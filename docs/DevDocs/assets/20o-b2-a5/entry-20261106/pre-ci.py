"""最终源码的编译器回归、格式与独立Runtime构建；失败立即保留并停止。"""
from pathlib import Path
import subprocess, sys
TASK=Path(__file__).resolve().parents[1]
SOURCE=TASK/'source'
cargo=['D:/Program Files/.cargo/bin/cargo.exe','+1.96.0']
manifest=['--locked','--manifest-path','core/rust/Cargo.toml']
changed=subprocess.check_output(['cmd','/c','git','diff','--name-only','bb7a856e','--','core/rust'],cwd=SOURCE).decode().splitlines()
new=subprocess.check_output(['cmd','/c','git','ls-files','--others','--exclude-standard','--','core/rust'],cwd=SOURCE).decode().splitlines()
files=sorted({name for name in changed+new if name.endswith('.rs')})
checks=[
 ('close-rustfmt',['D:/Program Files/.cargo/bin/rustup.exe','run','1.96.0','rustfmt','--check','--edition','2024','--config','skip_children=true',*files]),
 ('close-language-tests',cargo+['test',*manifest,'-p','xiao-driver','--test','20o_b2_a_contract','--','--nocapture']),
 ('close-package-tests',cargo+['test',*manifest,'-p','xiao-types','-p','xiao-ir','-p','xiao-bytecode','-p','xiao-runtime','-p','xiao-vm','-p','xiao-codegen-llvm','-p','xiao-diagnostics','--lib','--tests']),
 ('close-driver-unit-tests',cargo+['test',*manifest,'-p','xiao-driver','--lib']),
 ('close-runtime-release',cargo+['build',*manifest,'--release','-p','xiao-runtime']),
 ('close-doc-adapter',cargo+['build',*manifest,'-p','xiao-doc-coverage-rust']),
 ('close-b1-native-exact',cargo+['test',*manifest,'-p','xiao-driver','--test','20o_b1_probe','--','--ignored','b1_capabilities_match_native_golden_outputs','--exact','--nocapture']),
]
for name,argv in checks:
 result=subprocess.run([sys.executable,'-X','utf8','-u',str(TASK/'scripts/run-step.py'),name,'--',*argv],cwd=SOURCE)
 if result.returncode: raise SystemExit(result.returncode)
