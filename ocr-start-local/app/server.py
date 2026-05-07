import os
import tempfile
from pathlib import Path
from typing import Any

from fastapi import FastAPI, File, Form, HTTPException, UploadFile
from paddleocr import PaddleOCR


app = FastAPI(title="DevForge PaddleOCR", version="1.0.0")

_ocr_clients: dict[str, PaddleOCR] = {}


def get_ocr(lang: str) -> PaddleOCR:
    normalized_lang = lang.strip() or "ch"
    if normalized_lang not in _ocr_clients:
        _ocr_clients[normalized_lang] = PaddleOCR(
            lang=normalized_lang,
            use_doc_orientation_classify=False,
            use_doc_unwarping=False,
            use_textline_orientation=False,
        )
    return _ocr_clients[normalized_lang]


def to_jsonable(value: Any) -> Any:
    if isinstance(value, dict):
        return {str(key): to_jsonable(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [to_jsonable(item) for item in value]
    if hasattr(value, "tolist"):
        return value.tolist()
    if hasattr(value, "item"):
        return value.item()
    if isinstance(value, Path):
        return str(value)
    return value


def collect_texts(value: Any) -> list[str]:
    if isinstance(value, dict):
        texts: list[str] = []
        rec_texts = value.get("rec_texts")
        if isinstance(rec_texts, list):
            texts.extend(str(item) for item in rec_texts if item is not None)
        for item in value.values():
            texts.extend(collect_texts(item))
        return texts
    if isinstance(value, list):
        texts: list[str] = []
        for item in value:
            texts.extend(collect_texts(item))
        return texts
    return []


@app.get("/health")
def health() -> dict[str, str]:
    return {"status": "ok"}


@app.post("/ocr")
async def run_ocr(
    file: UploadFile = File(...),
    lang: str = Form("ch"),
) -> dict[str, Any]:
    suffix = Path(file.filename or "input.png").suffix or ".png"
    if suffix.lower() not in {".png", ".jpg", ".jpeg", ".bmp", ".tif", ".tiff", ".webp"}:
        raise HTTPException(status_code=400, detail="Only image files are supported by this endpoint.")

    content = await file.read()
    if not content:
        raise HTTPException(status_code=400, detail="Uploaded file is empty.")

    tmp_path = ""
    try:
        with tempfile.NamedTemporaryFile(delete=False, suffix=suffix) as tmp_file:
            tmp_file.write(content)
            tmp_path = tmp_file.name

        results = get_ocr(lang).predict(tmp_path)
        json_results = [to_jsonable(result.json) for result in results]
        lines = collect_texts(json_results)

        return {
            "filename": file.filename,
            "lang": lang,
            "text": "\n".join(lines),
            "lines": lines,
            "results": json_results,
        }
    finally:
        if tmp_path and os.path.exists(tmp_path):
            os.remove(tmp_path)
