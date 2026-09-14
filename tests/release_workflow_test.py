from pathlib import Path
import os
import stat
import subprocess
import tempfile
import unittest

import yaml


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW_PATH = ROOT / ".github" / "workflows" / "release.yml"
FAKE_GH = r'''#!/bin/sh
set -eu
printf '%s\n' "$*" >> "$FAKE_GH_LOG"
if [ "$1" = "api" ]; then
    endpoint="$2"
    case "$endpoint" in
        users/bot-user) response='{"type":"Bot"}' ;;
        users/api-failure) exit 1 ;;
        users/*) response='{"type":"User"}' ;;
        */git/ref/tags/*)
            case "${FAKE_TAG_MODE:-commit}" in
                commit) response='{"object":{"type":"commit","sha":"commit-sha"}}' ;;
                annotated) response='{"object":{"type":"tag","sha":"tag-sha"}}' ;;
                mismatch) response='{"object":{"type":"commit","sha":"other-sha"}}' ;;
                *) exit 1 ;;
            esac
            ;;
        */git/tags/tag-sha) response='{"object":{"sha":"commit-sha"}}' ;;
        */collaborators/admin/permission) response='{"role_name":"admin","permission":"admin"}' ;;
        */collaborators/maintainer/permission) response='{"role_name":"maintain","permission":"write"}' ;;
        */collaborators/write-only/permission) response='{"role_name":"write","permission":"write"}' ;;
        */collaborators/api-failure/permission) exit 1 ;;
        *) exit 1 ;;
    esac
    jq_filter=''
    previous=''
    for argument in "$@"; do
        if [ "$previous" = "--jq" ]; then
            jq_filter="$argument"
        fi
        previous="$argument"
    done
    if [ -n "$jq_filter" ]; then
        printf '%s' "$response" | jq -r "$jq_filter"
    else
        printf '%s\n' "$response"
    fi
    exit 0
fi
if [ "$1" = "release" ]; then
    case "$2" in
        view)
            test "${FAKE_RELEASE_EXISTS:-0}" = 1
            ;;
        create)
            test "${FAKE_CREATE_FAILURE:-0}" != 1
            ;;
        upload)
            test "${FAKE_UPLOAD_FAILURE:-0}" != 1
            ;;
        edit)
            test "${FAKE_PUBLISH_FAILURE:-0}" != 1
            ;;
        *) exit 2 ;;
    esac
    exit 0
fi
exit 2
'''


class ReleaseWorkflowTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = yaml.safe_load(WORKFLOW_PATH.read_text())
        cls.fake_root = Path(tempfile.mkdtemp(prefix="gh-auto-switcher-workflow-test-"))
        cls.fake_gh = cls.fake_root / "gh"
        cls.fake_gh.write_text(FAKE_GH)
        cls.fake_gh.chmod(cls.fake_gh.stat().st_mode | stat.S_IXUSR)

    @classmethod
    def tearDownClass(cls):
        for path in sorted(cls.fake_root.rglob("*"), reverse=True):
            if path.is_file() or path.is_symlink():
                path.unlink()
            else:
                path.rmdir()
        cls.fake_root.rmdir()

    @classmethod
    def step_run(cls, job, name):
        for step in cls.workflow["jobs"][job]["steps"]:
            if step.get("name") == name:
                return step["run"]
        raise AssertionError(f"step not found: {job}/{name}")

    @classmethod
    def base_env(cls):
        return {
            "PATH": f"{cls.fake_root}:/usr/bin:/bin:/sbin:/opt/homebrew/bin",
            "HOME": str(cls.fake_root),
            "FAKE_GH_LOG": str(cls.fake_root / "gh.log"),
            "GH_TOKEN": "test-token",
            "GITHUB_REPOSITORY": "beomjungil/gh-auto-switcher",
            "GITHUB_EVENT_NAME": "push",
            "GITHUB_REF": "refs/tags/v0.1.0",
            "GITHUB_REF_NAME": "v0.1.0",
            "GITHUB_SHA": "commit-sha",
            "ORIGINAL_ACTOR": "maintainer",
            "ACTOR": "maintainer",
            "TRIGGERING_ACTOR": "maintainer",
        }

    @classmethod
    def run_script(cls, script, env=None, cwd=ROOT):
        merged = cls.base_env()
        if env:
            merged.update(env)
        return subprocess.run(
            ["bash", "-c", script],
            cwd=cwd,
            env=merged,
            capture_output=True,
            text=True,
        )

    def test_workflow_is_locked_to_the_distribution_repository_and_release_permissions(self):
        self.assertEqual(self.workflow["permissions"]["contents"], "read")
        self.assertEqual(self.workflow["jobs"]["publish-assets"]["environment"]["name"], "release")
        self.assertEqual(
            set(self.workflow["jobs"]["publish-assets"]["needs"]),
            {"authorize", "test", "build"},
        )
        self.assertEqual(
            self.workflow["jobs"]["publish-assets"]["permissions"]["contents"],
            "write",
        )
        self.assertNotIn("pull_request_target", WORKFLOW_PATH.read_text())
        self.assertNotIn(
            "--repo",
            self.step_run("authorize", "Verify release context and actors"),
        )
        self.assertNotIn(
            "--clobber",
            self.step_run("publish-assets", "Upload release assets"),
        )
        self.assertIn(
            "GITHUB_REPOSITORY\" = \"beomjungil/gh-auto-switcher",
            self.step_run("authorize", "Verify release context and actors"),
        )
        self.assertIn(
            "github.triggering_actor",
            WORKFLOW_PATH.read_text(),
        )

        assets = {
            item["asset"]: item
            for item in self.workflow["jobs"]["build"]["strategy"]["matrix"]["include"]
        }
        self.assertEqual(
            set(assets),
            {
                "gh-auto-switcher-linux-amd64",
                "gh-auto-switcher-darwin-amd64",
                "gh-auto-switcher-darwin-arm64",
            },
        )
        self.assertNotIn("windows", " ".join(assets))
        for job in ("test", "build"):
            checkout = next(
                step
                for step in self.workflow["jobs"][job]["steps"]
                if step.get("uses", "").startswith("actions/checkout@")
            )
            self.assertEqual(checkout["with"]["ref"], "${{ github.sha }}")
            self.assertFalse(checkout["with"]["persist-credentials"])

        action_steps = (
            self.workflow["jobs"]["test"]["steps"]
            + self.workflow["jobs"]["build"]["steps"]
            + self.workflow["jobs"]["publish-assets"]["steps"]
        )
        for step in action_steps:
            if "uses" in step:
                self.assertRegex(step["uses"], r"@[0-9a-f]{40}$")

    def test_authorization_run_block_accepts_only_human_admin_or_maintain(self):
        script = self.step_run("authorize", "Verify release context and actors")
        cases = {
            "maintainer": ("maintainer", "maintainer", 0),
            "admin": ("admin", "admin", 0),
            "write-only": ("write-only", "maintainer", 1),
            "bot": ("bot-user", "maintainer", 1),
            "api-failure": ("api-failure", "maintainer", 1),
            "missing": ("", "maintainer", 1),
        }
        for name, (original, triggering, expected) in cases.items():
            with self.subTest(name=name):
                result = self.run_script(
                    script,
                    {"ORIGINAL_ACTOR": original, "TRIGGERING_ACTOR": triggering},
                )
                self.assertEqual(result.returncode, expected, result.stderr)

    def test_tag_identity_run_block_matches_lightweight_and_annotated_tags(self):
        script = self.step_run("publish-assets", "Verify tag identity")
        for name, env, expected in [
            ("lightweight", {}, 0),
            ("annotated", {"FAKE_TAG_MODE": "annotated"}, 0),
            ("mismatch", {"FAKE_TAG_MODE": "mismatch"}, 1),
            ("api failure", {"FAKE_TAG_MODE": "failure"}, 1),
            ("tag path", {"GITHUB_REF_NAME": "v0.1.0/unsafe"}, 1),
        ]:
            with self.subTest(name=name):
                result = self.run_script(script, env)
                self.assertEqual(result.returncode, expected)

    def test_publish_run_block_rechecks_initial_actor_and_rerun_actors(self):
        script = self.step_run("publish-assets", "Verify publishing actor")
        for name, env in {
            "authorized": {},
            "actor not maintainer": {"ACTOR": "write-only"},
            "triggering actor not maintainer": {"TRIGGERING_ACTOR": "write-only"},
            "bot": {"ACTOR": "bot-user"},
        }.items():
            with self.subTest(name=name):
                result = self.run_script(script, env)
                self.assertEqual(result.returncode, 0 if name == "authorized" else 1)

    def test_asset_validation_rejects_missing_extra_directory_symlink_and_empty_assets(self):
        script = self.step_run("publish-assets", "Verify assets and create checksums")
        expected = [
            "gh-auto-switcher-linux-amd64",
            "gh-auto-switcher-darwin-amd64",
            "gh-auto-switcher-darwin-arm64",
        ]
        with tempfile.TemporaryDirectory(prefix="gh-auto-switcher-assets-") as directory:
            dist = Path(directory)

            def write_assets(names=None, empty=None):
                for path in dist.iterdir():
                    if path.is_dir() and not path.is_symlink():
                        for child in path.iterdir():
                            child.unlink()
                        path.rmdir()
                    else:
                        path.unlink()
                for name in names or expected:
                    path = dist / name
                    path.write_bytes(b"binary" if name != empty else b"")

            write_assets()
            self.assertEqual(self.run_script(script, cwd=dist).returncode, 0)
            self.assertTrue((dist / "checksums.txt").stat().st_size > 0)

            for name, setup in {
                "missing": lambda: write_assets(expected[:2]),
                "extra": lambda: write_assets(expected + ["unexpected"]),
                "empty": lambda: write_assets(empty=expected[0]),
            }.items():
                with self.subTest(name=name):
                    setup()
                    self.assertNotEqual(self.run_script(script, cwd=dist).returncode, 0)

            write_assets()
            (dist / "unexpected").mkdir()
            self.assertNotEqual(self.run_script(script, cwd=dist).returncode, 0)

            write_assets()
            (dist / "unexpected").symlink_to(dist / expected[0])
            self.assertNotEqual(self.run_script(script, cwd=dist).returncode, 0)

            write_assets()
            (dist / ".unexpected").write_bytes(b"unexpected")
            self.assertNotEqual(self.run_script(script, cwd=dist).returncode, 0)

    def test_publish_blocks_create_draft_upload_explicit_assets_and_publish(self):
        create = self.step_run("publish-assets", "Create draft release")
        upload = self.step_run("publish-assets", "Upload release assets")
        publish = self.step_run("publish-assets", "Publish release")
        (self.fake_root / "gh.log").write_text("")
        with tempfile.TemporaryDirectory(prefix="gh-auto-switcher-publish-") as directory:
            workdir = Path(directory)
            dist = workdir / "dist"
            dist.mkdir()
            for asset in [
                "gh-auto-switcher-linux-amd64",
                "gh-auto-switcher-darwin-amd64",
                "gh-auto-switcher-darwin-arm64",
            ]:
                (dist / asset).write_bytes(b"binary")
            (dist / "checksums.txt").write_text("checksum\n")

            existing = self.run_script(create, {"FAKE_RELEASE_EXISTS": "1"})
            self.assertNotEqual(existing.returncode, 0)
            self.assertNotIn("release create", (self.fake_root / "gh.log").read_text())

            created = self.run_script(create, {"GITHUB_REF_NAME": "v0.1.0-rc.1"})
            self.assertEqual(created.returncode, 0, created.stderr)
            log = (self.fake_root / "gh.log").read_text()
            self.assertIn("--draft", log)
            self.assertIn("--prerelease", log)

            failed_upload = self.run_script(
                "\n".join([create, upload, publish]),
                {"FAKE_UPLOAD_FAILURE": "1"},
                cwd=workdir,
            )
            self.assertNotEqual(failed_upload.returncode, 0)
            self.assertNotIn("release edit", (self.fake_root / "gh.log").read_text())

            uploaded = self.run_script(upload, cwd=workdir)
            self.assertEqual(uploaded.returncode, 0, uploaded.stderr)
            log = (self.fake_root / "gh.log").read_text()
            for asset in [
                "dist/gh-auto-switcher-linux-amd64",
                "dist/gh-auto-switcher-darwin-amd64",
                "dist/gh-auto-switcher-darwin-arm64",
                "dist/checksums.txt",
            ]:
                self.assertIn(asset, log)
            self.assertNotIn("dist/*", log)

            published = self.run_script(
                "\n".join([create, upload, publish]),
                cwd=workdir,
            )
            self.assertEqual(published.returncode, 0, published.stderr)
            self.assertIn("release edit", (self.fake_root / "gh.log").read_text())


if __name__ == "__main__":
    unittest.main()
