"""按 B6-4 开工单的验证等级派生门禁结论与 COFF 完整性。"""


def derive_status(original_passed, fix_runner, engineering_passed, release_differential_open):
    """Windows 功能复验接受 UnverifiedCoffExports，但 B6-4 通过还要求 COFF 完整。"""
    windows = fix_runner["Windows"]
    followup_passed = (
        engineering_passed
        and all(fix_runner[host]["conclusion"] == "success" and fix_runner[host]["verification"] == "complete"
                for host in ("Linux", "macOS"))
        and windows["conclusion"] == "success"
        and windows["verification"] in ("complete", "unverified-coff-exports")
    )
    coff_complete = windows["verification"] == "complete"
    return {
        "b6_4_passed": original_passed and followup_passed and not release_differential_open and coff_complete,
        "original_passed": original_passed,
        "followup_passed": followup_passed,
        "windows_coff_verified": coff_complete,
        "22d_open": release_differential_open,
    }
