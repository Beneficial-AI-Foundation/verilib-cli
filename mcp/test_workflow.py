"""No network or real credentials: test argv, schema limits and timeout handling."""
import asyncio
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "lib"))
import runner
spec = importlib.util.spec_from_file_location("verilib_tools", ROOT / "verilib_cli_mcp/__main__.py")
tools = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tools)


class WorkflowTests(unittest.TestCase):
    def test_create_forwards_metadata_and_wait(self):
        with patch.object(tools, "run_cli", return_value={"ok": True}) as run:
            tools.verilib_repo_create("https://github.com/example/repo", "summary", 2, 2, 8,
                                     wait_for_ready=True, timeout_seconds=30, poll_interval_seconds=2)
            args = run.call_args.args[0]
            self.assertEqual(args[:2], ["repo", "create"])
            self.assertIn("--wait", args)
            self.assertEqual(run.call_args.kwargs["timeout_seconds"], 50)
            self.assertTrue(run.call_args.kwargs["json_output"])

    def test_init_deploy_status_and_wait(self):
        with patch.object(tools, "run_cli", return_value={}) as run:
            tools.verilib_init("42", execution_mode="docker")
            self.assertIn("docker", run.call_args.args[0])
            tools.verilib_deploy(wait_for_ready=True, accept_edits=True)
            self.assertIn("--wait", run.call_args.args[0])
            self.assertIn("--yes", run.call_args.args[0])
            tools.verilib_repo_status("42")
            self.assertEqual(run.call_args.args[0][:2], ["repo", "status"])
            tools.verilib_wait_for_ready("42")
            self.assertEqual(run.call_args.args[0][0], "wait-for-ready")

    def test_unset_base_does_not_override_project(self):
        self.assertEqual(tools._env(), {})
        self.assertEqual(tools._env("https://demo.example"), {"VERILIB_BASE_URL": "https://demo.example"})

    def test_runner_closes_stdin_and_parses_structured_errors(self):
        result = subprocess.CompletedProcess([], 1, json.dumps({"ok": False, "error": {"code": "REPO_NOT_READY"}}), "")
        with patch.object(runner, "resolve_binary", return_value="/fake/verilib"), patch.object(runner.subprocess, "run", return_value=result) as run:
            output = runner.run_cli(["deploy"], json_output=True, timeout_seconds=12)
            self.assertFalse(output["ok"])
            self.assertEqual(output["data"]["error"]["code"], "REPO_NOT_READY")
            self.assertEqual(run.call_args.kwargs["stdin"], subprocess.DEVNULL)
            self.assertEqual(run.call_args.kwargs["timeout"], 12)
            self.assertNotIn("shell", run.call_args.kwargs)

    def test_runner_timeout_is_not_success(self):
        with patch.object(runner, "resolve_binary", return_value="/fake/verilib"), patch.object(runner.subprocess, "run", side_effect=subprocess.TimeoutExpired("verilib", 1)):
            result = runner.run_cli(["repo", "create"], timeout_seconds=1)
            self.assertFalse(result["ok"])
            self.assertEqual(result["error"]["code"], "PROCESS_TIMEOUT")
            self.assertIn("unknown", result["error"]["message"])

    def test_actual_mcp_schema_and_validation(self):
        schemas = {tool.name: tool.input_schema for tool in asyncio.run(tools.server.list_tools())}
        create = schemas["verilib_repo_create"]["properties"]
        self.assertEqual(create["summary"]["maxLength"], 128)
        self.assertEqual(create["description"]["maxLength"], 512)
        self.assertEqual(create["timeout_seconds"]["minimum"], 1)
        self.assertIn("verilib_wait_for_ready", schemas)
        with patch.object(tools, "run_cli", return_value={"ok": True}) as run:
            asyncio.run(tools.server.call_tool("verilib_repo_status", {"repo_id": "42"}))
            run.assert_called_once()
        with patch.object(tools, "run_cli") as run:
            with self.assertRaises(Exception):
                asyncio.run(tools.server.call_tool("verilib_repo_create", {
                    "git_url":"https://github.com/example/repo", "summary":"a" * 129,
                    "language_id":2, "prooflanguage_id":2, "type_id":8}))
            run.assert_not_called()

    def test_wait_limits(self):
        for timeout, interval in [(0, 5), (10, 0), (86401, 5)]:
            with self.assertRaises(ValueError):
                tools._wait_args(True, timeout, interval)


if __name__ == "__main__":
    unittest.main()
