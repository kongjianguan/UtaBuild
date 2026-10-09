import argparse
import base64
import copy
import io
import json
import secrets
import subprocess
import sys
import threading
from datetime import datetime, timezone
from pathlib import Path

import uvicorn
from fastapi import BackgroundTasks, FastAPI, HTTPException, Request
from fastapi.responses import FileResponse, JSONResponse
from fastapi.staticfiles import StaticFiles
from jsonschema import ValidationError
from pydantic import BaseModel, ConfigDict, Field

from composition import EDIT_SCHEMA, empty_edits, validate_component_ids, validate_edits
from render import PROJECT_SCHEMA, TEACHING_SCHEMA, Renderer, canonical, file_hash, read_json, validate_inputs
from timing import aligned_project


class PreviewRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    edits: dict
    layout: str = Field(pattern="^(landscape|portrait)$")
    time_ms: float = Field(ge=0)


class ExportRequest(PreviewRequest):
    start_ms: float = Field(default=0, ge=0)
    end_ms: float | None = Field(default=None, gt=0)


class ArrangeRequest(PreviewRequest):
    lines: list[int] = Field(min_length=1)


def write_json(path, data):
    temporary = path.with_suffix(".pending.json")
    temporary.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def create_app(args):
    project_path = args.project.resolve(strict=True)
    teaching_path = args.teaching.resolve(strict=True)
    project = read_json(project_path, PROJECT_SCHEMA)
    teaching = read_json(teaching_path, TEACHING_SCHEMA)
    aligned = aligned_project(project)
    entries = validate_inputs(aligned, teaching)
    calibration_path = Path(project["calibration_path"]).resolve(strict=True)
    workspace = args.workspace.resolve()
    workspace.mkdir(parents=True, exist_ok=True)
    identity = {"project": str(project_path), "project_sha256": file_hash(project_path),
                "teaching": str(teaching_path), "teaching_sha256": file_hash(teaching_path),
                "cover_sha256": file_hash(project["background_path"]), "font": str(args.font.resolve(strict=True)),
                "calibration": str(calibration_path), "calibration_sha256": file_hash(calibration_path),
                "lyric_offset_ms": project["lyric_offset_ms"],
                "font_index": args.font_index}
    source_hashes = {path: file_hash(path) for path in [project_path, teaching_path, args.font.resolve(strict=True),
                                                    Path(project["audio_path"]), Path(project["background_path"]), calibration_path]}
    session_path = workspace / "session.json"
    if session_path.exists() and json.loads(session_path.read_text()) != identity:
        raise ValueError("编辑目录属于其他输入文件，请选择独立的 workspace")
    write_json(session_path, identity)
    edits_path = workspace / "edit.json"
    if not edits_path.exists():
        write_json(edits_path, empty_edits())
    read_json(edits_path, EDIT_SCHEMA)
    cache = {}
    jobs = {data["id"]: data for path in (workspace / "exports").glob("*/job.json")
            for data in [json.loads(path.read_text(encoding="utf-8"))]}
    lock = threading.Lock()
    edits_lock = threading.Lock()
    app = FastAPI(docs_url=None, redoc_url=None)
    token = secrets.token_urlsafe(32)

    @app.middleware("http")
    async def local_requests(request: Request, call_next):
        if request.method == "POST" and request.headers.get("x-editor-token") != token:
            return JSONResponse({"detail": "编辑会话令牌无效，请重新打开编辑器"}, status_code=403)
        return await call_next(request)

    @app.exception_handler(ValueError)
    @app.exception_handler(ValidationError)
    async def invalid_edit(request: Request, error):
        return JSONResponse({"detail": str(error)}, status_code=422)

    def validate_request(body):
        validate_edits(body.edits)
        validate_component_ids(body.edits, project, entries, canonical)
        for path, expected in source_hashes.items():
            if file_hash(path) != expected:
                raise ValueError(f"项目输入已经改变，请使用当前文件重新打开编辑器：{path}")
        if body.time_ms > project["duration_ms"]:
            raise ValueError("预览时间超出歌曲范围")

    def renderer(body):
        validate_request(body)
        key = json.dumps(body.edits, sort_keys=True, ensure_ascii=False)
        with lock:
            if body.layout not in cache or cache[body.layout][0] != key:
                rendered = Renderer(project, teaching, body.layout, args.font, args.font_index, edits=body.edits,
                                    base=cache[body.layout][1] if body.layout in cache else None)
                cache[body.layout] = (key, rendered)
            return cache[body.layout][1]

    @app.get("/api/session")
    def session():
        return {"title": project["title"], "artist": project["artist"], "duration_ms": project["duration_ms"],
                "lines": [{"id": line["id"], "text": line["text"], "start_ms": line["start_ms"], "end_ms": line["end_ms"]}
                          for line in aligned["lines"]], "edits": read_json(edits_path, EDIT_SCHEMA),
                "lyric_offset_ms": project["lyric_offset_ms"], "calibration": str(calibration_path),
                "calibration_sha256": source_hashes[calibration_path],
                "workspace": str(workspace), "token": token}

    @app.get("/api/cover")
    def cover():
        return FileResponse(project["background_path"])

    @app.get("/api/audio")
    def audio():
        return FileResponse(project["audio_path"])

    @app.post("/api/preview")
    def preview(body: PreviewRequest):
        r = renderer(body)
        with lock:
            nodes = r.preview_nodes(body.time_ms)
            buffer = io.BytesIO()
            r.frame(body.time_ms).save(buffer, format="PNG")
            return {"width": r.width, "height": r.height, "accent": r.accent_hex,
                    "image": "data:image/png;base64," + base64.b64encode(buffer.getvalue()).decode(),
                    "catalog": r.catalog(), **nodes}

    @app.post("/api/arrange")
    def arrange(body: ArrangeRequest):
        validate_request(body)
        if set(body.lines) - {line["id"] for line in project["lines"]}:
            raise ValueError("排版范围包含无效歌词场景")
        edits = copy.deepcopy(body.edits)
        offsets = edits["positions"][body.layout]
        for key in list(offsets):
            if any(key.startswith(f"line:{line}/") for line in body.lines):
                del offsets[key]
        measure = copy.deepcopy(edits)
        measure["positions"][body.layout] = {}
        with lock:
            r = Renderer(project, teaching, body.layout, args.font, args.font_index, edits=measure, natural=True,
                         base=cache[body.layout][1] if body.layout in cache else None)
        for scene in r.scenes:
            if scene["line_id"] in body.lines:
                edits["arrangements"][body.layout][str(scene["line_id"])] = r.arrangement(scene)
        return {"edits": edits}

    def persist_edits(edits):
        with edits_lock:
            write_json(edits_path, edits)
            return {"path": str(edits_path), "sha256": file_hash(edits_path)}

    @app.post("/api/save")
    def save(body: PreviewRequest):
        renderer(body)
        return persist_edits(body.edits)

    def export_job(job_id, folder, body):
        job = jobs[job_id]
        script = Path(__file__).parent
        output = folder / "video.mp4"
        command = [sys.executable, str(script / "render.py"), "--project", str(project_path),
                   "--teaching", str(teaching_path), "--layout", body.layout, "--font", str(args.font),
                   "--font-index", str(args.font_index), "--edits", str(folder / "edit.json"),
                   "--start-ms", str(body.start_ms), "--output", str(output)]
        if body.end_ms is not None:
            command += ["--end-ms", str(body.end_ms)]
        job["status"] = "rendering"
        with (folder / "render.log").open("w", encoding="utf-8") as log:
            process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=log, text=True)
            for line in process.stdout:
                log.write(line)
                log.flush()
                progress = json.loads(line)
                if "rendered_frames" in progress:
                    job["progress"] = progress["rendered_frames"] / progress["total_frames"]
            status = process.wait()
        if status != 0:
            job.update(status="failed", error=(folder / "render.log").read_text(encoding="utf-8"))
            write_json(folder / "job.json", job)
            return
        job.update(status="verifying", progress=1)
        command = [sys.executable, str(script / "verify.py"), "--manifest", str(output.with_suffix(".render.json")),
                   "--evidence-dir", str(folder / "evidence")]
        if body.start_ms == 0 and (body.end_ms is None or body.end_ms == project["duration_ms"]):
            command.append("--require-full-song")
        with (folder / "verify.log").open("w", encoding="utf-8") as log:
            status = subprocess.run(command, stdout=log, stderr=log).returncode
        if status != 0:
            job.update(status="failed", error=(folder / "verify.log").read_text(encoding="utf-8"))
        else:
            job.update(status="passed", video=str(output), report=str(folder / "evidence" / "verification.json"),
                       download=f"/api/exports/{job_id}/video", verification=f"/api/exports/{job_id}/report")
        write_json(folder / "job.json", job)

    @app.post("/api/export", status_code=202)
    def export(body: ExportRequest, tasks: BackgroundTasks):
        renderer(body)
        end = project["duration_ms"] if body.end_ms is None else body.end_ms
        if not 0 <= body.start_ms < end <= project["duration_ms"]:
            raise ValueError("导出时间必须位于完整歌曲范围内")
        if any(job["status"] in ["queued", "rendering", "verifying"] for job in jobs.values()):
            raise HTTPException(409, "正在导出视频，请等待当前导出完成")
        job_id = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S-") + secrets.token_hex(3)
        folder = workspace / "exports" / job_id
        folder.mkdir(parents=True)
        write_json(folder / "edit.json", body.edits)
        persist_edits(body.edits)
        jobs[job_id] = {"id": job_id, "status": "queued", "progress": 0, "layout": body.layout}
        tasks.add_task(export_job, job_id, folder, body)
        return jobs[job_id].copy()

    @app.get("/api/exports/{job_id}")
    def job(job_id: str):
        if job_id not in jobs:
            raise HTTPException(404, "导出任务不存在")
        return jobs[job_id].copy()

    @app.get("/api/exports/{job_id}/{kind}")
    def download(job_id: str, kind: str):
        if job_id not in jobs or jobs[job_id]["status"] != "passed":
            raise HTTPException(404, "导出尚未通过验证")
        if kind == "video":
            return FileResponse(jobs[job_id]["video"], filename=f"{project['title']}-{jobs[job_id]['layout']}.mp4")
        if kind == "report":
            return FileResponse(jobs[job_id]["report"], filename="verification.json")
        raise HTTPException(404, "产物不存在")

    app.mount("/", StaticFiles(directory=Path(__file__).parent / "editor", html=True), name="editor")
    return app


def main():
    parser = argparse.ArgumentParser(description="打开教学视频后期编辑器")
    parser.add_argument("--project", type=Path, required=True)
    parser.add_argument("--teaching", type=Path, required=True)
    parser.add_argument("--font", type=Path, required=True)
    parser.add_argument("--font-index", type=int, default=0)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--port", type=int, default=49603)
    args = parser.parse_args()
    uvicorn.run(create_app(args), host="127.0.0.1", port=args.port)


if __name__ == "__main__":
    main()
