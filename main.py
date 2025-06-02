from fastapi import FastAPI

app = FastAPI()

@app.get("/up")
async def health_check():
    return {"status": "ok"}