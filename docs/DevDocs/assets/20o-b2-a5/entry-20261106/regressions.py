"""在同一最终源码串行执行B1回归、VM精确点与完整workspace gates。"""
from pathlib import Path
import subprocess,sys
TASK=Path(__file__).resolve().parents[1]
SOURCE=TASK/'source'
EVIDENCE=SOURCE/'docs/DevDocs/assets/20o-b2-a5/entry-20261106'
cargo=['D:/Program Files/.cargo/bin/cargo.exe','+1.96.0']
manifest=['--locked','--manifest-path','core/rust/Cargo.toml']
checks=[
 ('close-b1-contract',cargo+['test',*manifest,'-p','xiao-driver','--test','20o_b1_imp_contract']),
 ('close-b1-resources',cargo+['test',*manifest,'-p','xiao-driver','--test','20o_b1_imp_resources']),
 ('close-b1-vm-exact',cargo+['test',*manifest,'-p','xiao-driver','--test','20o_b1_probe','--','b1_capabilities_have_vm_golden_outputs','--exact','--nocapture']),
 ('close-a1-vm-exact',cargo+['test',*manifest,'-p','xiao-driver','--test','20o_b2_a1_identity','a1_numeric_values_and_errors_have_vm_golden_outputs','--','--exact','--nocapture']),
 ('close-a1-native-exact',cargo+['test',*manifest,'-p','xiao-driver','--test','20o_b2_a1_identity','a1_numeric_values_and_errors_match_native_golden_outputs','--','--ignored','--exact','--nocapture']),
 ('close-workspace-check',cargo+['check',*manifest,'--workspace']),
 ('close-benchmarks-check',cargo+['check','--locked','--manifest-path','tests/benchmarks/Cargo.toml']),
 ('close-b1-guards',['D:/Program Files/PowerShell/7/pwsh.exe','-NoProfile','-File','tools/gates/b1-probe-guards.ps1','-BashPath','D:/msys64/usr/bin/bash.exe','-ReportPath',str(EVIDENCE/'close-b1-guards-report.json')]),
]
for name,argv in checks:
 result=subprocess.run([sys.executable,'-X','utf8','-u',str(TASK/'scripts/run-step.py'),name,'--',*argv],cwd=SOURCE)
 if result.returncode: raise SystemExit(result.returncode)
