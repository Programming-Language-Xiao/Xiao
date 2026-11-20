"""按 B6-4 开工单的验证等级派生门禁结论与 COFF 完整性。"""


def derive_status(original_passed, fix_runner, engineering_passed, release_differential_open):
    """后续复验要求三平台完整验证；Windows COFF 未验证时 followup_passed 为 false。"""
    windows = fix_runner["Windows"]
    coff_complete = windows["verification"] == "complete"
    followup_passed = (
        engineering_passed
        and all(fix_runner[host]["conclusion"] == "success" and fix_runner[host]["verification"] == "complete"
                for host in ("Linux", "macOS"))
        and windows["conclusion"] == "success"
        and coff_complete
    )
    return {
        "b6_4_passed": original_passed and followup_passed and not release_differential_open,
        "original_passed": original_passed,
        "followup_passed": followup_passed,
        "windows_coff_verified": coff_complete,
        "22d_open": release_differential_open,
    }
