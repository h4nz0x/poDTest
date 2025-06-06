use ansi_term::Colour;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::path::Path;
use std::os::unix::process::ExitStatusExt;
use std::process::{self, Stdio};
use sysinfo::{Disks, System};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{self, Duration};

#[derive(Debug, Parser)]
pub(crate) struct Cli {
    #[arg(long, default_value = "Dockerfile")]
    dockerfile_path: String,
    #[arg(long, default_value_t = 1200)]
    build_timeout: u64,
    #[arg(long, action)]
    hot_fix: bool,
    #[arg(long, default_value = "/up")]
    health_check_path: String,
    #[arg(long, default_value_t = 8000)]
    port: u16,
    #[arg(long, default_value_t = 10)]
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
    let image_name = std::env::var("IMAGE_NAME").unwrap_or_else(|_| "docker-test:latest".to_string());
    let container_name = image_name.replace(":", "_").replace("/", "_");

    let build_context = Path::new(&cli.dockerfile_path)
        .parent()
        .ok_or_else(|| anyhow!("Invalid Dockerfile path: {}", cli.dockerfile_path))?
        .to_str()
        .ok_or_else(|| anyhow!("Invalid path encoding"))?
        .to_string();

    if cli.hot_fix {
        print_header("BUILD IMAGE");
        println!("{}", Colour::Yellow.paint("Hot-fix mode: Building image only, skipping tests..."));
        let _ = build_image(
            &image_name,
            &build_context,
            &cli.dockerfile_path,
            cli.build_timeout,
        )
        .await?;
        println!(
            "{}",
            Colour::Green.paint(format!(
                "Image {} built successfully in hot-fix mode",
                image_name
            ))
        );
        return Ok(());
    }

    print_header("BUILD IMAGE");
    let (image_id, container_name, image_name) = match test_docker_container(&cli).await {
        Ok(result) => result,
        Err(e) => {
            eprintln!(
                "{}",
                Colour::Red.paint(format!(
                    "Container failed to start (\"{} did not return 200OK\"): {}",
                    cli.health_check_path, e
                ))
            );
            if !container_name.is_empty() {
                println!(
                    "{}",
                    Colour::Yellow.paint("Fetching container logs for:")
                );
                println!("{}", container_name);
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
                    println!(
                        "{} {}",
                        Colour::Yellow.paint("No container logs available for:"),
                        container_name
                    );
                } else {
                    println!(
                        "{}",
                        Colour::Yellow.paint("Container logs (stdout):")
                    );
                    println!("{}", stdout);
                    if !stderr.is_empty() {
                        eprintln!(
                            "{}",
                            Colour::Red.paint("Container logs (stderr):")
                        );
                        eprintln!("{}", stderr);
                    }
                }
            }
            eprintln!("{}", Colour::Red.paint("Container failed to start"));
            print_header("CLEANUP");
            cleanup_docker("", &container_name, &image_name).await?;
            return Err(anyhow!("Failed to start container {}: {}", container_name, e));
        }
    };

    print_header("CLEANUP");
    cleanup_docker(&image_id, &container_name, &image_name).await?;

    println!(
        "{}",
        Colour::Green.paint(format!(
            "Container started successfully and {} endpoint returned 200 OK",
            cli.health_check_path
        ))
    );
    Ok(())
}

fn check_system_resources() -> Result<()> {
    let mut system = System::new_all();
    system.refresh_all();

    if let Some(disk) = Disks::new_with_refreshed_list().list().first() {
        let total = disk.total_space() as f64 / 1024.0 / 1024.0 / 1024.0;
        let used = (disk.total_space() - disk.available_space()) as f64 / 1024.0 / 1024.0 / 1024.0;
        let free = disk.available_space() as f64 / 1024.0 / 1024.0 / 1024.0;
        println!(
            "{} {:.2} GB, {:.2} GB, {:.2} GB",
            Colour::Yellow.paint("Disk space: Total ="),
            total, used, free
        );
        if free < 1.0 {
            println!(
                "{}",
                Colour::Blue.paint("Warning: Low disk space (<1 GB free)")
            );
        }
    }

    let free_memory = system.free_memory() as f64 / 1024.0 / 1024.0;
    println!(
        "{} {:.2} MB",
        Colour::Yellow.paint("Available memory:"),
        free_memory
    );
    if free_memory < 500.0 {
        println!(
            "{}",
            Colour::Blue.paint("Warning: Low memory (<500 MB available)")
        );
    }

    Ok(())
}

async fn check_docker_daemon_status() -> Result<()> {
    let output = Command::new("docker")
        .arg("version")
        .output()
        .await
        .context("Failed to run docker version")?;
    println!(
        "{}",
        Colour::Yellow.paint("Docker version:")
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        println!(
            "{}",
            Colour::Red.paint("Docker version stderr:")
        );
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }

    let output = Command::new("docker")
        .arg("info")
        .output()
        .await
        .context("Failed to run docker info")?;
    println!(
        "{}",
        Colour::Yellow.paint("Docker info:")
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));

    Ok(())
}

async fn test_docker_container(cli: &Cli) -> Result<(String, String, String)> {
    let image_name =
        std::env::var("IMAGE_NAME").unwrap_or_else(|_| "docker-test:latest".to_string());
    let health_check_url = format!("http://localhost:3000{}", cli.health_check_path);

    if !Path::new(&cli.dockerfile_path).exists() {
        println!(
            "{} {}",
            Colour::Red.paint("Dockerfile not found at:"),
            cli.dockerfile_path
        );
        return Err(anyhow!(
            "Dockerfile not found at {}",
            cli.dockerfile_path
        ));
    }
    println!(
        "{} {}",
        Colour::Yellow.paint("Dockerfile found at:"),
        cli.dockerfile_path
    );

    let build_context = Path::new(&cli.dockerfile_path)
        .parent()
        .ok_or_else(|| anyhow!("Invalid Dockerfile path: {}", cli.dockerfile_path))?
        .to_str()
        .ok_or_else(|| anyhow!("Invalid path encoding"))?;

    let dockerignore_path = format!("{}/.dockerignore", build_context);
    if !Path::new(&dockerignore_path).exists() {
        println!(
            "{} {}",
            Colour::Blue.paint("Warning: .dockerignore not found at:"),
            dockerignore_path
        );
        println!(". Consider creating one with: .git, venv, __pycache__, *.log");
    } else {
        let contents = std::fs::read_to_string(&dockerignore_path)?;
        println!(
            "{}",
            Colour::Yellow.paint(".dockerignore contents:")
        );
        println!("{}", contents);
    }

    let image_id = build_image(
        &image_name,
        build_context,
        &cli.dockerfile_path,
        cli.build_timeout,
    )
    .await?;

    print_header("RUN CONTAINER");
    let container_name = run_container(&image_name, cli.port, 3000).await?;

    print_header("TEST CONTAINER");
    println!("{}", Colour::Yellow.paint("Testing Container..."));
    time::sleep(Duration::from_secs(10)).await;

    let client = reqwest::Client::new();
    println!(
        "{} {}",
        Colour::Yellow.paint("Testing health check endpoint:"),
        health_check_url
    );
    let mut attempts = 0;
    let max_attempts = cli.health_check_interval;
    while attempts < max_attempts {
        attempts += 1;
        println!(
            "{} {}/{}",
            Colour::Yellow.paint("Health check attempt:"),
            attempts, max_attempts
        );
        match time::timeout(
            Duration::from_secs(cli.health_check_timeout),
            client.get(&health_check_url).send(),
        )
        .await
        {
            Ok(Ok(response)) => {
                let status = response.status();
                let content = response.text().await?.chars().take(100).collect::<String>();
                println!(
                    "{} status_code={}, content={}",
                    Colour::Yellow.paint("Health check response:"),
                    status, content
                );
                if status.is_success() {
                    println!(
                        "{} {} endpoint returned 200",
                        Colour::Green.paint("Health check passed:"),
                        cli.health_check_path
                    );
                    return Ok((image_id, container_name, image_name));
                }
                if attempts == max_attempts {
                    return Err(anyhow!("Health check failed with status {}", status));
                }
            }
            Ok(Err(e)) => {
                println!(
                    "{} {}/{} failed: {}",
                    Colour::Red.paint("Health check attempt:"),
                    attempts, max_attempts, e
                );
                if attempts == max_attempts {
                    return Err(anyhow!("Health check failed: {}", e));
                }
            }
            Err(_) => {
                println!(
                    "{} {}/{} timed out after {} seconds",
                    Colour::Yellow.paint("Health check attempt:"),
                    attempts, max_attempts, cli.health_check_timeout
                );
                if attempts == max_attempts {
                    return Err(anyhow!(
                        "Health check timed out after {} attempts",
                        max_attempts
                    ));
                }
            }
        }
        time::sleep(Duration::from_secs(1)).await;
    }

    Err(anyhow!("Health check failed after {} attempts", max_attempts))
}

async fn build_image(
    image_name: &str,
    build_context: &str,
    dockerfile_path: &str,
    build_timeout: u64,
) -> Result<String> {
    println!(
        "{} {}",
        Colour::Yellow.paint("Starting Docker image build for:"),
        image_name
    );
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

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Failed to capture stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("Failed to capture stderr"))?;

    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            println!("{} {}", Colour::Yellow.paint("Build log:"), line);
        }
    });

    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            println!("{} {}", Colour::Yellow.paint("Build log:"), line);
        }
    });

    let status = time::timeout(Duration::from_secs(build_timeout), child.wait())
        .await
        .map_err(|_| anyhow!("Docker build timed out after {} seconds", build_timeout))?;
    let status = status?;
    if !status.success() {
        return Err(anyhow!("Docker build failed with status: {}", status));
    }

    stdout_task.await.unwrap_or(());
    stderr_task.await.unwrap_or(());

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
        return Err(anyhow!("Failed to get image ID"));
    }
    println!(
        "{} {} with ID: {}",
        Colour::Green.paint("Docker image built successfully:"),
        image_name, image_id
    );
    Ok(image_id)
}

async fn run_container(image_name: &str, internal_port: u16, external_port: u16) -> Result<String> {
    println!(
        "{} {}",
        Colour::Yellow.paint("Checking for port conflicts on:"),
        external_port
    );
    let port_check = Command::new("netstat")
        .args(["-tuln"])
        .output()
        .await
        .context("Failed to check port availability")?;
    let port_output = String::from_utf8_lossy(&port_check.stdout);
    if port_output.contains(&format!(":{}", external_port)) {
        return Err(anyhow!("Port {} is already in use", external_port));
    }

    println!(
        "{} {}",
        Colour::Yellow.paint("Starting container from image:"),
        image_name
    );
    let container_name = image_name.replace(":", "_").replace("/", "_");
    println!(
        "{} {}",
        Colour::Yellow.paint("Assigning container name:"),
        container_name
    );

    Command::new("docker")
        .arg("rm")
        .arg("-f")
        .arg(&container_name)
        .output()
        .await
        .ok();

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
        .arg("PYTHONIOENCODING=utf8")
        .arg("--name")
        .arg(&container_name)
        .arg("-p")
        .arg(format!("{}:{}", external_port, internal_port))
        .arg(image_name)
        .output()
        .await
        .context(format!("Failed to start container: {}", image_name))?;
    let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if container_id.is_empty() {
        return Err(anyhow!("Failed to start container: no container ID returned"));
    }
    println!(
        "{} {}",
        Colour::Green.paint("Container started:"),
        container_name
    );
    Ok(container_name)
}

async fn cleanup_docker(image_id: &str, container_name: &str, image_name: &str) -> Result<()> {
    println!(
        "{} container_name=\"{}\", image_name=\"{}\", image_id=\"{}\"",
        Colour::Yellow.paint("Cleanup started:"),
        container_name, image_name, image_id
    );

    let daemon_check = Command::new("docker").arg("info").output().await;
    if let Err(e) = daemon_check {
        println!(
            "{} {}. Ensure Docker is running and user has permissions (try 'sudo' or add user to 'docker' group).",
            Colour::Red.paint("Docker daemon check failed:"),
            e
        );
        return Err(anyhow!("Docker daemon inaccessible: {}", e));
    }
    let daemon_output = daemon_check.unwrap();
    if !daemon_output.status.success() {
        println!(
            "{} {}",
            Colour::Red.paint("Docker info failed:"),
            String::from_utf8_lossy(&daemon_output.stderr)
        );
        return Err(anyhow!("Docker daemon error"));
    }

    println!("{}", Colour::Yellow.paint("Cleaning up BuildKit containers..."));
    let buildkit_output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "-q",
            "--filter",
            "ancestor=moby/buildkit:buildx-stable-1",
        ])
        .args(["--filter", "name=buildx_buildkit_"])
        .output()
        .await
        .context("Failed to list BuildKit containers")?;
    let buildkit_output_str = String::from_utf8_lossy(&buildkit_output.stdout);
    let buildkit_container_ids = buildkit_output_str
        .trim()
        .lines()
        .collect::<Vec<_>>();
    if !buildkit_container_ids.is_empty() {
        println!(
            "{} {:?}",
            Colour::Yellow.paint("Found BuildKit containers:"),
            buildkit_container_ids
        );
        for id in buildkit_container_ids {
            let stop_output = Command::new("docker")
                .arg("stop")
                .arg(id)
                .output()
                .await
                .context(format!("Failed to stop BuildKit container {}", id))?;
            if stop_output.status.success() {
                println!(
                    "{} {}",
                    Colour::Yellow.paint("Stopped BuildKit container:"),
                    id
                );
            } else {
                println!(
                    "{} {}: {}",
                    Colour::Red.paint("Failed to stop BuildKit container:"),
                    id,
                    String::from_utf8_lossy(&stop_output.stderr)
                );
            }

            let rm_output = Command::new("docker")
                .arg("rm")
                .arg("-f")
                .arg(id)
                .output()
                .await
                .context(format!("Failed to remove BuildKit container {}", id))?;
            if rm_output.status.success() {
                println!(
                    "{} {}",
                    Colour::Yellow.paint("Removed BuildKit container:"),
                    id
                );
            } else {
                println!(
                    "{} {}: {}",
                    Colour::Red.paint("Failed to remove BuildKit container:"),
                    id,
                    String::from_utf8_lossy(&rm_output.stderr)
                );
            }
        }
    } else {
        println!("{}", Colour::Yellow.paint("No BuildKit containers found."));
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
        println!(
            "{} {} exists: {}",
            Colour::Yellow.paint("Container:"),
            container_name, container_exists
        );

        if container_exists {
            println!(
                "{} {}",
                Colour::Yellow.paint("Stopping container:"),
                container_name
            );
            let output = Command::new("docker")
                .arg("stop")
                .arg(container_name)
                .output()
                .await
                .context("Failed to stop container")?;
            println!(
                "{}",
                Colour::Yellow.paint("Container stop output:")
            );
            println!("{}", String::from_utf8_lossy(&output.stdout));
            if !output.stderr.is_empty() {
                println!(
                    "{}",
                    Colour::Red.paint("Container stop error:")
                );
                println!("{}", String::from_utf8_lossy(&output.stderr));
            }
            if !output.status.success() {
                println!(
                    "{} {}: {}",
                    Colour::Red.paint("Failed to stop container:"),
                    container_name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            println!(
                "{} {}",
                Colour::Yellow.paint("Removing container:"),
                container_name
            );
            let output = Command::new("docker")
                .args(["rm", "-f"])
                .arg(container_name)
                .output()
                .await
                .context(format!("Failed to remove container: {}", container_name))?;
            println!(
                "{}",
                Colour::Yellow.paint("Container remove output:")
            );
            println!("{}", String::from_utf8_lossy(&output.stdout));
            if !output.stderr.is_empty() {
                println!(
                    "{}",
                    Colour::Red.paint("Container remove error:")
                );
                println!("{}", String::from_utf8_lossy(&output.stderr));
            }
            if !output.status.success() {
                println!(
                    "{} {}: {}",
                    Colour::Red.paint("Failed to remove container:"),
                    container_name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        } else {
            println!(
                "{} {} does not exist, skipping removal.",
                Colour::Yellow.paint("Container:"),
                container_name
            );
        }
    }

    if !image_name.is_empty() || !image_id.is_empty() {
        let image_exists = Command::new("docker")
            .args(["images", "-q", image_id])
            .output()
            .await
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);
        println!(
            "{} {} exists: {}",
            Colour::Yellow.paint("Image ID:"),
            image_id, image_exists
        );

        if image_exists {
            println!(
                "{} {}",
                Colour::Yellow.paint("Removing image by ID:"),
                image_id
            );
            let output = Command::new("docker")
                .args(["rmi", "-f", image_id])
                .output()
                .await
                .context(format!("Failed to remove image by ID: {}", image_id))?;
            println!(
                "{} {}",
                Colour::Yellow.paint("Image remove output (ID:"),
                image_id
            );
            println!("{}", String::from_utf8_lossy(&output.stdout));
            if !output.stderr.is_empty() {
                println!(
                    "{} {}",
                    Colour::Red.paint("Image remove error (ID:"),
                    image_id
                );
                println!("{}", String::from_utf8_lossy(&output.stderr));
            }
            if !output.status.success() {
                println!(
                    "{} {}: {}",
                    Colour::Red.paint("Failed to remove image by ID:"),
                    image_id,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        let name_exists = Command::new("docker")
            .args(["images", "-q", image_name])
            .output()
            .await
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);
        println!(
            "{} {} exists: {}",
            Colour::Yellow.paint("Image name:"),
            image_name, name_exists
        );

        if name_exists {
            println!(
                "{} {}",
                Colour::Yellow.paint("Removing image by name:"),
                image_name
            );
            let output = Command::new("docker")
                .args(["rmi", "-f", image_name])
                .output()
                .await
                .context(format!("Failed to remove image by name: {}", image_name))?;
            println!(
                "{} {}",
                Colour::Yellow.paint("Image remove output (name:"),
                image_name
            );
            println!("{}", String::from_utf8_lossy(&output.stdout));
            if !output.stderr.is_empty() {
                println!(
                    "{} {}",
                    Colour::Red.paint("Image remove error (name:"),
                    image_name
                );
                println!("{}", String::from_utf8_lossy(&output.stderr));
            }
            if !output.status.success() {
                println!(
                    "{} {}: {}",
                    Colour::Red.paint("Failed to remove image by name:"),
                    image_name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        println!("{}", Colour::Yellow.paint("Removing image docker-test:latest..."));
        let output = Command::new("docker")
            .args(["rmi", "-f", "docker-test:latest"])
            .output()
            .await
            .context("Failed to remove image docker-test:latest")?;
        println!(
            "{}",
            Colour::Yellow.paint("Image remove output (docker-test:latest):")
        );
        println!("{}", String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            println!(
                "{}",
                Colour::Red.paint("Image remove error (docker-test:latest):")
            );
            println!("{}", String::from_utf8_lossy(&output.stderr));
        }
        if !output.status.success() {
            println!(
                "{} {}",
                Colour::Red.paint("Failed to remove image docker-test:latest:"),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    println!("{}", Colour::Yellow.paint("Cleaning up dangling images..."));
    let dangling_output = Command::new("docker")
        .args(["images", "-f", "dangling=true", "-q"])
        .output()
        .await
        .context("Failed to list dangling images")?;
    let dangling_str = String::from_utf8_lossy(&dangling_output.stdout);
    let dangling_image_ids = dangling_str.trim().lines().collect::<Vec<_>>();
    if !dangling_image_ids.is_empty() {
        println!(
            "{} {:?}",
            Colour::Yellow.paint("Found dangling images:"),
            dangling_image_ids
        );
        let output = Command::new("docker")
            .args(["rmi", "-f"])
            .args(&dangling_image_ids)
            .output()
            .await
            .context("Failed to remove dangling images")?;
        println!(
            "{}",
            Colour::Yellow.paint("Dangling images remove output:")
        );
        println!("{}", String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            println!(
                "{}",
                Colour::Red.paint("Dangling images remove error:")
            );
            println!("{}", String::from_utf8_lossy(&output.stderr));
        }
        if !output.status.success() {
            println!(
                "{} {}",
                Colour::Red.paint("Failed to remove dangling images:"),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    } else {
        println!("{}", Colour::Yellow.paint("No dangling images found."));
    }

    println!("{}", Colour::Yellow.paint("Pruning dangling images..."));
    let prune_output = Command::new("docker")
        .args(["image", "prune", "-f"])
        .output()
        .await
        .context("Failed to prune dangling images")?;
    println!(
        "{}",
        Colour::Yellow.paint("Image prune output:")
    );
    println!("{}", String::from_utf8_lossy(&prune_output.stdout));
    if !prune_output.stderr.is_empty() {
        println!(
            "{}",
            Colour::Red.paint("Image prune error:")
        );
        println!("{}", String::from_utf8_lossy(&prune_output.stderr));
    }

    let container_still_exists = Command::new("docker")
        .args(["ps", "-a", "-q"])
        .output()
        .await
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    let image_still_exists = Command::new("docker")
        .args(["images", "-q", image_name])
        .output()
        .await
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    let default_image_still_exists = Command::new("docker")
        .args(["images", "-q", "docker-test:latest"])
        .output()
        .await
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    let dangling_still_exist = Command::new("docker")
        .args(["images", "-f", "dangling=true", "-q"])
        .output()
        .await
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    if container_still_exists || image_still_exists || default_image_still_exists || dangling_still_exist {
        println!(
            "{} containers_exist={}, image_exists={}, default_image_exists={}, dangling_exists={}",
            Colour::Red.paint("Cleanup failed:"),
            container_still_exists,
            image_still_exists,
            default_image_still_exists,
            dangling_still_exist
        );
        if container_still_exists {
            let ps_output = Command::new("docker")
                .args(["ps", "-a"])
                .output()
                .await
                .unwrap_or_else(|_| process::Output {
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                    status: process::ExitStatus::from_raw(1),
                });
            println!(
                "{}",
                Colour::Red.paint("Remaining containers:")
            );
            println!("{}", String::from_utf8_lossy(&ps_output.stdout));
        }
        if image_still_exists || default_image_still_exists || dangling_still_exist {
            let images_output = Command::new("docker")
                .args(["images"])
                .output()
                .await
                .unwrap_or_else(|_| process::Output {
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                    status: process::ExitStatus::from_raw(1),
                });
            println!(
                "{}",
                Colour::Red.paint("Remaining images:")
            );
            println!("{}", String::from_utf8_lossy(&images_output.stdout));
        }
        return Err(anyhow!(
            "Failed to clean up: containers={}, images={}, default_image={}, dangling={}",
            container_still_exists,
            image_still_exists,
            default_image_still_exists,
            dangling_still_exist
        ));
    }

    println!("{}", Colour::Green.paint("Cleanup completed successfully"));
    Ok(())
}