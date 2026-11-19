"""B5-0只读审计：检查来源证据有效，但不把阻塞写成阶段通过。"""
import argparse
from io import BytesIO
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import shutil
import zipfile

sys.dont_write_bytecode = True
from facts import (ROOT, REPOSITORY, ENTRY, PRODUCT, B1, B1_ROOTS, RUNS, REQUIRED, candidate,
                   describe, docs_snapshot, fixed_paths, git, inventory, load, require, sha, source_facts, tree_digest)


def raw_object(root, row):
    path = root / row["path"]
    require(path.is_file(), "B5已登记文件缺失")
    raw = path.read_bytes()
    require(describe(raw) == {key: row[key] for key in ["bytes", "sha256"]}
            and row["path"].startswith("objects/sha256/" + sha(raw)[:2] + "/" + sha(raw)), "B5内容对象摘要不符")
    return raw


def receipt(root, row):
    require(row["pid"] > 0 and row["cwd"] == str(REPOSITORY)
            and row["started_utc"] < row["finished_utc"], "B5来源查询回执不符")
    return raw_object(root, row["stdout"]), raw_object(root, row["stderr"])


def inspect_zip(raw):
    with zipfile.ZipFile(BytesIO(raw)) as archive:
        return [{"path": member.filename, "bytes": member.file_size, "sha256": sha(archive.read(member))}
                for member in archive.infolist() if not member.is_dir()]


def check(root=ROOT, repository=REPOSITORY, sealed=True):
    before = tree_digest(root)
    if sealed:
        require((root / "evidence.json").is_file(), "B5证据清单缺失")
    if (root / "evidence.json").exists():
        expected = load(root / "evidence.json")
        require(all((root / name).is_file() for name in expected["files"]), "B5已登记文件缺失")
        require(expected == {"files": inventory(root)}, "B5证据清单不符")
    report = load(root / "preflight.json")
    require(report["source_facts"] == source_facts(repository), "B5入口身份不符")
    require(report["evidence_valid"] is True and report["stage_passed"] is False
            and report["blocked"] is True and report["blocker_code"] == "B5-PREFLIGHT-001"
            and report["product_started"] is False and report["b5_closed"] is False and report["b6_started"] is False
            and report["missing_required"] == REQUIRED, "B5阻塞状态冒进")
    require(report["source_paths"]["b1_fixed"] == fixed_paths(repository, B1, B1_ROOTS)
            and report["source_paths"]["entry_fixed"] == fixed_paths(repository, ENTRY, B1_ROOTS), "B5固定来源路径不符")
    history_raw, error = receipt(root, report["history_query"])
    require(report["history_query"]["exit_code"] == 0 and not error
            and report["history_query"]["argv"] == ["cmd", "/c", "git", "log", "--all", "--pretty=format:", "--name-only", "--", *B1_ROOTS]
            and report["source_paths"]["all_local_history"] == sorted(set(history_raw.decode("utf-8").splitlines()) - {""}),
            "B5历史路径查询不符")
    binary = {key: [path for path in paths if candidate(path)] for key, paths in report["source_paths"].items()}
    require(report["binary_candidates"] == binary and not any(binary.values()), "B5发现B1产物候选未处理")
    expected_zips = [path for path in report["source_paths"]["entry_fixed"] if path.endswith(".zip")]
    require([row["path"] for row in report["local_archives"]] == expected_zips, "B5本地归档清单不完整")
    for row in report["local_archives"]:
        raw = git(repository, "show", ENTRY + ":" + row["path"])
        require(describe(raw) == {key: row[key] for key in ["bytes", "sha256"]} and row["members"] == inspect_zip(raw)
                and not any(candidate(member["path"]) for member in row["members"]), "B5本地归档成员不符")
    require([row["run_id"] for row in report["remote_queries"]] == RUNS, "B5远端查询范围不符")
    for run in report["remote_queries"]:
        raw, error = receipt(root, run["receipt"])
        require(run["receipt"]["argv"] == ["gh", "api", "repos/Programming-Language-Xiao/Xiao/actions/runs/" + run["run_id"] + "/artifacts?per_page=100"]
                and run["receipt"]["exit_code"] == 0 and not error and json.loads(raw) == run["listing"],
                "B5远端归档列表不符")
        listing = run["listing"]
        require(listing["total_count"] == len(listing["artifacts"]) == len(run["archives"]), "B5远端归档未完整核对")
        for archived, artifact in zip(run["archives"], listing["artifacts"]):
            raw, error = receipt(root, archived["receipt"])
            require(archived["artifact_id"] == artifact["id"] and archived["name"] == artifact["name"]
                    and archived["receipt"]["argv"] == ["gh", "api", "repos/Programming-Language-Xiao/Xiao/actions/artifacts/" + str(artifact["id"]) + "/zip"]
                    and archived["receipt"]["exit_code"] == 0 and not error
                    and raw_object(root, archived["archive"]) == raw and inspect_zip(raw) == archived["members"]
                    and not any(candidate(member["path"]) for member in archived["members"]), "B5远端归档成员不符")
            require(not artifact.get("digest") or artifact["digest"] == "sha256:" + sha(raw), "B5远端归档摘要不符")
    require(report["archive_binary_candidates"] == report["remote_binary_candidates"] == [], "B5产物候选被忽略")
    raw, error = receipt(root, report["b4_5_audit"])
    require(report["b4_5_audit"]["exit_code"] == 0 and not error
            and json.loads(raw) == {"passed": True, "b4_5_closed": True, "b4_closed": True,
                                   "language_available": True, "product_commit": PRODUCT,
                                   "source_commit": "6e172c1ecf8b3c4f58cf0c8c18b0aa3ab8b91012"}, "B5上游审计回执不符")
    require(report["b4_artifacts_not_b1"] == [path for path in fixed_paths(repository, ENTRY, ["docs/DevDocs/assets"]) if candidate(path)],
            "B5过渡产物身份不符")
    if sealed:
        docs = load(root / "final-docs.json")
        raw, error = receipt(root, docs["receipt"])
        require(docs["receipt"]["exit_code"] == 0 and not error and json.loads(raw)["passed"] is True
                and docs["receipt"]["argv"] == ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"]
                and docs["docs_before"] == docs["docs_after"]
                and docs["fixed_files"] == docs_snapshot(repository, docs["docs_commit"])["canonical_files"]
                and docs["docs_before"]["canonical_files"] == docs["fixed_files"], "B5最终文档门禁不符")
        for path in root.iterdir():
            if path.suffix in {".py", ".md"}:
                relative = path.relative_to(repository).as_posix() if root == ROOT else ROOT.relative_to(repository).as_posix() + "/" + path.name
                require(sha(path.read_bytes().replace(b"\r\n", b"\n")) == docs["fixed_files"].get(relative),
                        "B5最终工具未定稿")
        review = load(root / "self-negative.json")
        expected_reasons = {"baseline": None, "manifest": "B5证据清单不符",
                            "missing": "B5已登记文件缺失", "status": "B5阻塞状态冒进"}
        require(review["passed"] is True and [row["case"] for row in review["cases"]] == list(expected_reasons),
                "B5反例报告不完整")
        for row in review["cases"]:
            reason = expected_reasons[row["case"]]
            require(row["reason"] == reason and row["stderr"].strip() == (reason or "")
                    and row["exit_code"] == (0 if reason is None else 1) and row["read_only"] is True
                    and re.fullmatch("[0-9a-f]{64}", row["before_sha256"])
                    and row["before_sha256"] == row["after_sha256"], "B5反例报告原因或摘要不符")
    require(before == tree_digest(root), "B5审计改写证据")
    return {"evidence_valid": True, "stage_passed": False, "blocked": True, "b5_closed": False,
            "runtime_abi": "1.18", "codegen_version": 21, "missing_required": REQUIRED}


def negative_cases(root=ROOT, repository=REPOSITORY):
    rows = []
    with tempfile.TemporaryDirectory(prefix="b5-preflight-negative-") as temporary:
        for name in ["baseline", "manifest", "missing", "status"]:
            copy = Path(temporary) / name
            shutil.copytree(root, copy)
            reason = None
            if name == "manifest":
                (copy / "preflight.json").write_bytes(b"tampered")
                reason = "B5证据清单不符"
            elif name == "missing":
                (copy / ".gitattributes").unlink()
                reason = "B5已登记文件缺失"
            elif name == "status":
                report = load(copy / "preflight.json")
                report["stage_passed"] = True
                (copy / "preflight.json").write_text(json.dumps(report, ensure_ascii=False), encoding="utf-8")
                (copy / "evidence.json").write_text(json.dumps({"files": inventory(copy)}, ensure_ascii=False), encoding="utf-8")
                reason = "B5阻塞状态冒进"
            before = tree_digest(copy)
            run = subprocess.run([sys.executable, "-B", "-X", "utf8", str(Path(__file__).resolve()),
                                  "--root", str(copy), "--repository-root", str(repository), "--unsealed"],
                                 capture_output=True, encoding="utf-8")
            after = tree_digest(copy)
            require(before == after and run.returncode == (0 if reason is None else 1)
                    and run.stderr.strip() == (reason or ""), "B5反例拒绝原因或只读性不符：" + name)
            rows.append({"case": name, "exit_code": run.returncode, "reason": reason,
                         "stderr": run.stderr, "read_only": True, "before_sha256": before, "after_sha256": after})
    return {"passed": True, "cases": rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--unsealed", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--self-negative", action="store_true")
    args = parser.parse_args()
    if args.self_negative:
        print(json.dumps(negative_cases(args.root, args.repository_root), ensure_ascii=False))
        return
    print(json.dumps(check(args.root, args.repository_root, sealed=not args.unsealed), ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, AssertionError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
