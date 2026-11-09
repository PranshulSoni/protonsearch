"""Model/provider configuration for the sidecar.

Priority for every knob: environment variable > optional JSON config file >
built-in default. The config file is `%APPDATA%/protonsearch/v2/agent.json` on
Windows (or the path in `PROTONSEARCH_AGENT_CONFIG`); it may contain
`{"model": "...", "baseUrl": "...", "testModel": false, "maxSteps": 8,
"runTimeoutS": 300}`. It must NEVER contain an API key: keys come from the
environment or the OS secret store (see secretstore.py).

This module deliberately does not perform any network I/O. Provider calls are
made by pydantic-ai only when a run executes with a real model; the test model
performs no I/O at all.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .secretstore import api_key_from_env_or_store

DEFAULT_MAX_STEPS = 8
DEFAULT_RUN_TIMEOUT_S = 300.0
TEST_MODEL_LABEL = "test:scripted"


class ModelNotConfigured(RuntimeError):
    pass


@dataclass(frozen=True)
class AgentConfig:
    model: str | None = None
    base_url: str | None = None
    test_model: bool = False
    max_steps: int = DEFAULT_MAX_STEPS
    run_timeout_s: float = DEFAULT_RUN_TIMEOUT_S


def _default_config_path() -> Path:
    override = os.environ.get("PROTONSEARCH_AGENT_CONFIG")
    if override:
        return Path(override)
    appdata = os.environ.get("APPDATA")
    if appdata:
        return Path(appdata) / "protonsearch" / "v2" / "agent.json"
    return Path.home() / ".config" / "protonsearch" / "agent.json"


def _read_config_file() -> dict[str, Any]:
    path = _default_config_path()
    try:
        if not path.is_file():
            return {}
        raw = path.read_text(encoding="utf-8")
        data = json.loads(raw)
    except (OSError, ValueError):
        # A broken optional config must never prevent the sidecar from starting;
        # the host turns a missing model into a clear not_configured error.
        return {}
    return data if isinstance(data, dict) else {}


def _env_bool(name: str) -> bool | None:
    value = os.environ.get(name)
    if value is None or value == "":
        return None
    return value.strip().lower() in ("1", "true", "yes", "on")


def load_config() -> AgentConfig:
    file_data = _read_config_file()

    def pick(name: str, file_key: str) -> Any:
        value = os.environ.get(name)
        return value if value not in (None, "") else file_data.get(file_key)

    model = pick("PROTONSEARCH_AGENT_MODEL", "model")
    base_url = pick("PROTONSEARCH_AGENT_BASE_URL", "baseUrl")

    test_model = _env_bool("PROTONSEARCH_AGENT_TEST_MODEL")
    if test_model is None:
        test_model = file_data.get("testModel") is True

    max_steps_raw = pick("PROTONSEARCH_AGENT_MAX_STEPS", "maxSteps")
    try:
        max_steps = int(max_steps_raw) if max_steps_raw is not None else DEFAULT_MAX_STEPS
    except (TypeError, ValueError):
        max_steps = DEFAULT_MAX_STEPS
    max_steps = max(1, min(max_steps, 32))

    timeout_raw = pick("PROTONSEARCH_AGENT_RUN_TIMEOUT_S", "runTimeoutS")
    try:
        run_timeout_s = float(timeout_raw) if timeout_raw is not None else DEFAULT_RUN_TIMEOUT_S
    except (TypeError, ValueError):
        run_timeout_s = DEFAULT_RUN_TIMEOUT_S
    run_timeout_s = max(5.0, min(run_timeout_s, 3600.0))

    return AgentConfig(
        model=str(model) if model else None,
        base_url=str(base_url) if base_url else None,
        test_model=bool(test_model),
        max_steps=max_steps,
        run_timeout_s=run_timeout_s,
    )


def _apply_provider_environment(config: AgentConfig, model: str) -> None:
    """Bridge our generic knobs onto the provider env pydantic-ai reads.

    Only OpenAI-compatible base URLs are mapped; other providers keep their
    own documented env vars (ANTHROPIC_API_KEY, ...). Nothing is logged.
    """
    if config.base_url:
        os.environ.setdefault("OPENAI_BASE_URL", config.base_url)
    api_key = api_key_from_env_or_store()
    if not api_key:
        return
    key_var: str | None = None
    if model.startswith(("openai:", "openai/", "openai-compatible:")) or "openai" in model.split(":", 1)[0]:
        key_var = "OPENAI_API_KEY"
    elif model.startswith(("anthropic:", "anthropic/")):
        key_var = "ANTHROPIC_API_KEY"
    if key_var and not os.environ.get(key_var):
        os.environ[key_var] = api_key


def build_model(config: AgentConfig, request_model: str | None, request_context_paths: tuple[str, ...], run_id: str):
    """Return (model_ref, label). Raises ModelNotConfigured without a model."""
    if config.test_model:
        from .test_model import build_scripted_test_model

        return build_scripted_test_model(run_id, request_model or "", request_context_paths), TEST_MODEL_LABEL

    model_name = request_model or config.model
    if not model_name:
        raise ModelNotConfigured(
            "no model configured; set PROTONSEARCH_AGENT_MODEL or use the test model "
            "(PROTONSEARCH_AGENT_TEST_MODEL=1)"
        )
    _apply_provider_environment(config, model_name)
    return model_name, model_name
