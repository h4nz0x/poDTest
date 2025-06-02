use ansi_term::Colour;
  use anyhow::{Context, Result};
  use clap::Parser;
  use std::path::Path;
  use std::process::Stdio;
  use sysinfo::{DiskExt, System, SystemExt};
  use tokio::io::{AsyncBufReadExt, BufReader};
  use tokio::process::Command;
  use tokio::time::{timeout, Duration};

  #[derive(Parser)]
  struct Cli {
      #[arg(long, default_value = "Dockerfile")]
      dockerfile_path: String,
      #[arg(long, default_value_t = 1200)]
      build_timeout: u64,
      #[arg(long, action)]
      hot_fix: bool,
      #[arg(long, default_value = "/up")]
      health_check_path: String,
      #[arg(long, default_value_t = 80)]
      port: u16,
      #[arg(long, default_value_t = 5)]
      health_check_timeout: u64,
      #[arg(long, default_value_t = 5)]
      health_check_interval: u32,
  }

  fn print_header(step: &str) {
      println!(
          "{}",
          Colour::Yellow.paint(format!(
              "######################################################\n#                                                    #\n#{:^50}#\n#                                                    #\n######################################################",
              step.to_uppercase()
          ))
      );
  }

  #[tokio::main]
  async fn main() -> Result<()> {
      let cli = Cli::parse();
      println!("{}", Colour::Yellow.paint("Starting Docker test script..."));

      check_system_resources()?;
      check_docker_daemon_status().await?;

      println!("{}", Colour::Yellow.paint("Starting build and test process..."));
      let image_name = std::env::var("IMAGE_NAME").unwrap_or_else(|_| "my-app:latest".to_string());
      let container_name = image_name.replace(":", "_").replace("/", "_");

      let build_context = Path::new(&cli.dockerfile_path)
          .parent()
          .ok_or_else(|| anyhow::anyhow!("Invalid Dockerfile path: {}", cli.dockerfile_path))?
          .to_str()
          .ok_or_else(|| anyhow::anyhow!("Invalid path encoding"))?
          .to_string();

      if cli.hot_fix {
          print_header("BUILD IMAGE");
          println!("{}", Colour::Yellow.paint("Hot-fix mode: Building image only, skipping tests..."));
          let _ = build_image(&image_name, &build_context, &cli.dockerfile_path, cli.build_timeout).await?;
          println!("{}", Colour::Green.paint(format!("Image {} built successfully in hot-fix mode", image_name)));
          return Ok(());
      }

      print_header("BUILD IMAGE");
      let (id, c_name, i_name) = match test_docker_container(&cli).await {
          Ok(result) => result,
          Err(_) => {
              eprintln!("{}", Colour::Red.paint(format!("container failed to start (\"{} did not return 200OK\")", cli.health_check_path)));
              if !container_name.is_empty() {
                  println!("{}", Colour::Yellow.paint(format!("Fetching container logs for {}...", container_name)));
                  let logs_output = Command::new("docker")
                      .arg("logs")
                      .arg("--tail")
                      .arg("100")
                      .arg(&container_name)
                      .output()
                      .await
                      .context("Failed to fetch container logs")?;
                  let stdout = String::from_utf8_lossy(&logs_output.stdout);
                  let stderr = String::from_utf8_lossy(&logs_output.stderr);
                  if stdout.is_empty() && stderr.is_empty() {
                      println!("{}", Colour::Yellow.paint(format!("No container logs available for {}.", container_name)));
                  } else {
                      println!("{}", Colour::Yellow.paint(format!("Container logs (stdout):\n{}", stdout)));
                      if !stderr.is_empty() {
                          eprintln!("{}", Colour::Red.paint(format!("Container logs (stderr):\n{}", stderr)));
                      }
                  }
              }
              eprintln!("{}", Colour::Red.paint("Container Failed to become healthy"));
              print_header("CLEANUP");
              let _ = cleanup_docker(&String::new(), &container_name, &image_name).await;
              return Err(anyhow::anyhow!("Container failed to become healthy"));
          }
      };
      let image_id = id;
      let container_name = c_name;
      let image_name = i_name;

      print_header("CLEANUP");
      cleanup_docker(&image_id, &container_name, &image_name).await?;

      println!("{}", Colour::Green.paint(format!("Container started and {} endpoint returned 200 OK successfully", cli.health_check_path)));
      Ok(())
  }

  fn check_system_resources() -> Result<()> {
      let mut sys = System::new_all();
      sys.refresh_all();

      if let Some(disk) = sys.disks().first() {
          let total = disk.total_space() as f64 / 1024.0 / 1024.0 / 1024.0;
          let used = (disk.total_space() - disk.available_space()) as f64 / 1024.0 / 1024.0 / 1024.0;
          let free = disk.available_space() as f64 / 1024.0 / 1024.0 / 1024.0;
          println!("{}", Colour::Yellow.paint(format!("Disk space: Total={:.2} GB, Used={:.2} GB, Free={:.2} GB", total, used, free)));
          if free < 1.0 {
              println!("{}", Colour::Yellow.paint("Warning: Low disk space (<1 GB free)"));
          }
      }

      let free_memory = sys.free_memory() as f64 / 1024.0 / 1024.0;
      println!("{}", Colour::Yellow.paint(format!("Available memory: {:.2} MB", free_memory)));
      if free_memory < 500.0 {
          println!("{}", Colour::Yellow.paint("Warning: Low memory (<500 MB available)"));
      }

      Ok(())
  }

  async fn check_docker_daemon_status() -> Result<()> {
      let output = Command::new("docker")
          .arg("version")
          .output()
          .await
          .context("Failed to run docker version")?;
      println!("{}", Colour::Yellow.paint(format!("Docker version:\n{}", String::from_utf8_lossy(&output.stdout))));
      if !output.stderr.is_empty() {
          eprintln!("{}", Colour::Red.paint(format!("Docker version stderr:\n{}", String::from_utf8_lossy(&output.stderr))));
      }

      let output = Command::new("docker")
          .arg("info")
          .output()
          .await
          .context("Failed to run docker info")?;
      println!("{}", Colour::Yellow.paint(format!("Docker info:\n{}", String::from_utf8_lossy(&output.stdout))));
      if !output.stderr.is_empty() {
          eprintln!("{}", Colour::Red.paint(format!("Docker info stderr:\n{}", String::from_utf8_lossy(&output.stderr))));
      }

      Ok(())
  }

  async fn test_docker_container(cli: &Cli) -> Result<(String, String, String)> {
      let image_name = std::env::var("IMAGE_NAME").unwrap_or_else(|_| "my-app:latest".to_string());
      let health_check_url = format!("http://localhost:3000{}", cli.health_check_path);

      if !Path::new(&cli.dockerfile_path).exists() {
          eprintln!("{}", Colour::Red.paint(format!("Dockerfile not found at {}", cli.dockerfile_path)));
          return Err(anyhow::anyhow!("Dockerfile not found at {}", cli.dockerfile_path));
      }
      println!("{}", Colour::Yellow.paint(format!("Dockerfile found at {}", cli.dockerfile_path)));

      let build_context = Path::new(&cli.dockerfile_path)
          .parent()
          .ok_or_else(|| anyhow::anyhow!("Invalid Dockerfile path: {}", cli.dockerfile_path))?
          .to_str()
          .ok_or_else(|| anyhow::anyhow!("Invalid path encoding"))?;

      let dockerignore_path = format!("{}/.dockerignore", build_context);
      if !Path::new(&dockerignore_path).exists() {
          println!("{}", Colour::Yellow.paint(format!("Warning: .dockerignore not found at {}. Consider creating one with: .git, venv, __pycache__, *.log", dockerignore_path)));
      } else {
          let contents = std::fs::read_to_string(&dockerignore_path)?;
          println!("{}", Colour::Yellow.paint(format!(".dockerignore contents:\n{}", contents)));
      }

      let image_id = build_image(&image_name, build_context, &cli.dockerfile_path, cli.build_timeout).await?;

      print_header("RUN CONTAINER");
      let container_name = run_container(&image_name, cli.port, 3000).await?;

      print_header("TEST CONTAINER");
      println!("{}", Colour::Yellow.paint("Waiting for container to be ready (1 second)..."));
      tokio::time::sleep(Duration::from_secs(1)).await;

      let client = reqwest::Client::new();
      println!("{}", Colour::Yellow.paint(format!("Testing health check endpoint: {}", health_check_url)));
      let mut attempts = 0;
      let max_attempts = cli.health_check_interval;
      while attempts < max_attempts {
          attempts += 1;
          println!("{}", Colour::Yellow.paint(format!("Health check attempt {}/{}", attempts, max_attempts)));
          match timeout(
              Duration::from_secs(cli.health_check_timeout),
              client.get(&health_check_url).send(),
          )
          .await
          {
              Ok(Ok(response)) => {
                  let status = response.status();
                  let content = response.text().await?.chars().take(100).collect::<String>();
                  println!("{}", Colour::Yellow.paint(format!("Health check response: status_code={}, content={}", status, content)));
                  if status.is_success() {
                      println!("{}", Colour::Green.paint(format!("Health check passed: {} endpoint returned 200", cli.health_check_path)));
                      return Ok((image_id, container_name, image_name));
                  }
                  if attempts == max_attempts {
                      return Err(anyhow::anyhow!("Health check failed with status {}", status));
                  }
              }
              Ok(Err(e)) => {
                  eprintln!("{}", Colour::Red.paint(format!("Health check attempt {}/{} failed: {}", attempts, max_attempts, e)));
                  if attempts == max_attempts {
                      return Err(anyhow::anyhow!("Health check failed: {}", e));
                  }
              }
              Err(_) => {
                  eprintln!("{}", Colour::Red.paint(format!("Health check attempt {}/{} timed out after {} seconds", attempts, max_attempts, cli.health_check_timeout)));
                  if attempts == max_attempts {
                      return Err(anyhow::anyhow!("Health check timed out after {} attempts", max_attempts));
                  }
              }
          }
          tokio::time::sleep(Duration::from_secs(1)).await;
      }

      Err(anyhow::anyhow!("Health check failed after {} attempts", max_attempts))
  }

  async fn build_image(image_name: &str, build_context: &str, dockerfile_path: &str, build_timeout: u64) -> Result<String> {
      println!("{}", Colour::Yellow.paint(format!("Starting Docker image build for {}...", image_name)));
      let mut child = Command::new("docker")
          .arg("build")
          .arg("-t")
          .arg(image_name)
          .arg("-f")
          .arg(dockerfile_path)
          .arg(build_context)
          .stdout(Stdio::piped())
          .stderr(Stdio::piped())
          .spawn()
          .context("Failed to start docker build")?;

      let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;
      let stderr = child.stderr.take().ok_or_else(|| anyhow::anyhow!("Failed to capture stderr"))?;

      let stdout_task = tokio::spawn(async move {
          let mut reader = BufReader::new(stdout).lines();
          loop {
              match reader.next_line().await {
                  Ok(Some(line)) => println!("{}", Colour::Yellow.paint(format!("Build log: {}", line))),
                  Ok(None) => break,
                  Err(e) => {
                      eprintln!("{}", Colour::Red.paint(format!("Build stdout error: {}", e)));
                      break;
                  }
              }
          }
      });

      let stderr_task = tokio::spawn(async move {
          let mut reader = BufReader::new(stderr).lines();
          loop {
              match reader.next_line().await {
                  Ok(Some(line)) => println!("{}", Colour::Yellow.paint(format!("Build log: {}", line))),
                  Ok(None) => break,
                  Err(e) => {
                      eprintln!("{}", Colour::Red.paint(format!("Build stderr error: {}", e)));
                      break;
                  }
              }
          }
      });

      let status = timeout(Duration::from_secs(build_timeout), child.wait())
          .await
          .map_err(|_| anyhow::anyhow!("Docker build timed out after {} seconds", build_timeout))?;
      let status = status?;
      if !status.success() {
          return Err(anyhow::anyhow!("Docker build failed with status: {}", status));
      }

      stdout_task.await.ok();
      stderr_task.await.ok();

      let output = Command::new("docker")
          .arg("images")
          .arg("--no-trunc")
          .arg("--format")
          .arg("{{.ID}}")
          .arg(image_name)
          .output()
          .await
          .context("Failed to get image ID")?;
      let image_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
      if image_id.is_empty() {
          return Err(anyhow::anyhow!("Failed to get image ID"));
      }
      println!("{}", Colour::Green.paint(format!("Docker image {} built successfully with ID: {}", image_name, image_id)));
      Ok(image_id)
  }

  async fn run_container(image_name: &str, internal_port: u16, external_port: u16) -> Result<String> {
      println!("{}", Colour::Yellow.paint(format!("Starting container from image {}...", image_name)));
      let container_name = image_name.replace(":", "_").replace("/", "_");
      println!("{}", Colour::Yellow.paint(format!("Assigning container name: {}", container_name)));

      // Remove existing container with the same name, if any
      Command::new("docker")
          .arg("rm")
          .arg("-f")
          .arg(&container_name)
          .output()
          .await
          .ok(); // Ignore errors if container doesn't exist

      let output = Command::new("docker")
          .arg("run")
          .arg("-d")
          .arg("--log-driver")
          .arg("json-file")
          .arg("--log-opt")
          .arg("max-size=10m")
          .arg("-e")
          .arg("PYTHONUNBUFFERED=1")
          .arg("-e")
          .arg("PYTHONIOENCODING=utf-8")
          .arg("--name")
          .arg(&container_name)
          .arg("-p")
          .arg(format!("{}:{}", external_port, internal_port))
          .arg(image_name)
          .output()
          .await
          .context("Failed to start container")?;
      let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
      if container_id.is_empty() {
          return Err(anyhow::anyhow!("Failed to start container"));
      }
      println!("{}", Colour::Green.paint(format!("Container started with name: {}", container_name)));
      Ok(container_name)
  }

  async fn cleanup_docker(image_id: &str, container_name: &str, _image_name: &str) -> Result<()> {
      println!("{}", Colour::Yellow.paint(format!("Cleanup started: container_name='{}', image_name='{}', image_id='{}'.", container_name, _image_name, image_id)));

      // Check Docker daemon status
      let daemon_check = Command::new("docker").arg("info").output().await;
      if let Err(e) = daemon_check {
          eprintln!("{}", Colour::Red.paint(format!("Docker daemon check failed: {}. Ensure Docker is running and user has permissions (try 'sudo' or add user to 'docker' group).", e)));
          return Err(anyhow::anyhow!("Docker daemon inaccessible: {}", e));
      }
      let daemon_output = daemon_check.unwrap();
      if !daemon_output.status.success() {
          eprintln!("{}", Colour::Red.paint(format!("Docker info failed: stderr: {}. Ensure Docker is running and accessible.", String::from_utf8_lossy(&daemon_output.stderr))));
          return Err(anyhow::anyhow!("Docker daemon error"));
      }

      if !container_name.is_empty() {
          let container_exists = Command::new("docker")
              .arg("ps")
              .arg("-a")
              .arg("-q")
              .arg("--filter")
              .arg(format!("name={}", container_name))
              .output()
              .await
              .map(|o| !o.stdout.is_empty())
              .unwrap_or(false);
          println!("{}", Colour::Yellow.paint(format!("Container '{}' exists: {}", container_name, container_exists)));

          if container_exists {
              println!("{}", Colour::Yellow.paint(format!("Stopping container '{}'.", container_name)));
              let output = Command::new("docker")
                  .arg("stop")
                  .arg(container_name)
                  .output()
                  .await
                  .context("Failed to stop container")?;
              println!("{}", Colour::Yellow.paint(format!("Container stop output:\n{}", String::from_utf8_lossy(&output.stdout))));
              if !output.stderr.is_empty() {
                  eprintln!("{}", Colour::Red.paint(format!("Container stop stderr:\n{}", String::from_utf8_lossy(&output.stderr))));
              }
              if !output.status.success() {
                  eprintln!("{}", Colour::Red.paint(format!("Failed to stop container (exit code {}): stderr: {}", output.status, String::from_utf8_lossy(&output.stderr))));
              }

              println!("{}", Colour::Yellow.paint(format!("Removing container '{}'.", container_name)));
              let output = Command::new("docker")
                  .arg("rm")
                  .arg("-f")
                  .arg(container_name)
                  .output()
                  .await
                  .context("Failed to remove container")?;
              println!("{}", Colour::Yellow.paint(format!("Container remove output:\n{}", String::from_utf8_lossy(&output.stdout))));
              if !output.stderr.is_empty() {
                  eprintln!("{}", Colour::Red.paint(format!("Container remove stderr:\n{}", String::from_utf8_lossy(&output.stderr))));
              }
              if !output.status.success() {
                  eprintln!("{}", Colour::Red.paint(format!("Failed to remove container (exit code {}): stderr: {}", output.status, String::from_utf8_lossy(&output.stderr))));
              }
          } else {
              println!("{}", Colour::Yellow.paint(format!("Container '{}' does not exist, skipping removal.", container_name)));
          }
      }

      if !_image_name.is_empty() {
          let image_exists = Command::new("docker")
              .arg("images")
              .arg("-q")
              .arg(_image_name)
              .output()
              .await
              .map(|o| !o.stdout.is_empty())
              .unwrap_or(false);
          println!("{}", Colour::Yellow.paint(format!("Image '{}' exists: {}", _image_name, image_exists)));

          if image_exists && !image_id.is_empty() {
              println!("{}", Colour::Yellow.paint(format!("Removing image '{}' (ID: '{}').", _image_name, image_id)));
              let output = Command::new("docker")
                  .arg("rmi")
                  .arg("-f")
                  .arg(image_id)
                  .output()
                  .await
                  .context("Failed to remove image by ID")?;
              println!("{}", Colour::Yellow.paint(format!("Image remove output (ID):\n{}", String::from_utf8_lossy(&output.stdout))));
              if !output.stderr.is_empty() {
                  eprintln!("{}", Colour::Red.paint(format!("Image remove stderr (ID):\n{}", String::from_utf8_lossy(&output.stderr))));
              }
              if !output.status.success() {
                  eprintln!("{}", Colour::Red.paint(format!("Failed to remove image by ID (exit code {}): stderr: {}", output.status, String::from_utf8_lossy(&output.stderr))));
              }
          }

          if image_exists {
              println!("{}", Colour::Yellow.paint(format!("Removing image '{}' by name.", _image_name)));
              let output = Command::new("docker")
                  .arg("rmi")
                  .arg("-f")
                  .arg(_image_name)
                  .output()
                  .await
                  .context("Failed to remove image by name")?;
              println!("{}", Colour::Yellow.paint(format!("Image remove output (name):\n{}", String::from_utf8_lossy(&output.stdout))));
              if !output.stderr.is_empty() {
                  eprintln!("{}", Colour::Red.paint(format!("Image remove stderr (name):\n{}", String::from_utf8_lossy(&output.stderr))));
              }
              if !output.status.success() {
                  eprintln!("{}", Colour::Red.paint(format!("Failed to remove image by name (exit code {}): stderr: {}", output.status, String::from_utf8_lossy(&output.stderr))));
              }
          }
      }

      // Verify cleanup
      let container_still_exists = Command::new("docker")
          .arg("ps")
          .arg("-a")
          .arg("-q")
          .arg("--filter")
          .arg(format!("name={}", container_name))
          .output()
          .await
          .map(|o| !o.stdout.is_empty())
          .unwrap_or(false);
      let image_still_exists = Command::new("docker")
          .arg("images")
          .arg("-q")
          .arg(_image_name)
          .output()
          .await
          .map(|o| !o.stdout.is_empty())
          .unwrap_or(false);

      if container_still_exists || image_still_exists {
          eprintln!("{}", Colour::Red.paint(format!("Cleanup failed: container_exists={}, image_exists={}", container_still_exists, image_still_exists)));
          return Err(anyhow::anyhow!("Failed to clean up container {} or image {}", container_name, _image_name));
      }

      println!("{}", Colour::Green.paint("Cleanup completed successfully"));
      Ok(())
  }