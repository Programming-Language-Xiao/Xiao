from pathlib import Path
import subprocess,sys
step=Path(__file__).with_name('run-step.py');m=['--locked','--manifest-path','core/rust/Cargo.toml'];base=['cargo','+1.96.0'];driver=['test',*m,'-p','xiao-driver','--test']
commands=[
 ('build-adapters',[*base,'build',*m,'-p','xiao-doc-coverage-rust','-p','xiao-diagnostics']),
 ('b1-contract',[*base,*driver,'20o_b1_imp_contract']),('b1-resources',[*base,*driver,'20o_b1_imp_resources']),
 ('b2-resources',[*base,*driver,'20o_b2_a_resources','--test','20o_b2_a_performance_protocol']),
 ('b2-language',[*base,*driver,'20o_b2_a_contract','--','--nocapture']),
 ('package-tests',[*base,'test',*m,'-p','xiao-types','-p','xiao-ir','-p','xiao-bytecode','-p','xiao-runtime','-p','xiao-vm','-p','xiao-codegen-llvm','-p','xiao-diagnostics','--lib','--tests']),
 ('driver-unit',[*base,'test',*m,'-p','xiao-driver','--lib']),
 ('b1-vm-exact',[*base,*driver,'20o_b1_probe','--','b1_capabilities_have_vm_golden_outputs','--exact','--nocapture']),
 ('b1-native-exact',[*base,*driver,'20o_b1_probe','--','--ignored','b1_capabilities_match_native_golden_outputs','--exact','--nocapture']),
 ('a1-vm-exact',[*base,*driver,'20o_b2_a1_identity','a1_numeric_values_and_errors_have_vm_golden_outputs','--','--exact','--nocapture']),
 ('a1-native-exact',[*base,*driver,'20o_b2_a1_identity','a1_numeric_values_and_errors_match_native_golden_outputs','--','--ignored','--exact','--nocapture']),
 ('workspace-check',[*base,'check',*m,'--workspace']),('benchmarks-check',[*base,'check','--locked','--manifest-path','tests/benchmarks/Cargo.toml']),
 ('workspace-clippy',[*base,'clippy',*m,'--workspace','--all-targets','--','-D','warnings']),
 ('workspace-fmt',['rustup','run','1.96.0','cargo','fmt','--manifest-path','core/rust/Cargo.toml','--all','--','--check']),
 ('b1-guards',['pwsh','-NoProfile','-File','tools/gates/b1-probe-guards.ps1','-BashPath','D:/msys64/usr/bin/bash.exe','-ReportPath',str(step.parents[1]/'source/docs/DevDocs/assets/20o-b2-a7/entry-20261107/b1-guards-report.json')]),
 ('docs-initial',['bun','tools/repo-check/src/cli.ts','all'])]
for name,argv in commands:
 if Path(argv[0]).name=='cargo' and argv[2]=='fmt':
  # fmt查询/格式不接受--locked，执行器仅对构建/测试/检查要求锁。
  pass
 p=subprocess.run([sys.executable,'-X','utf8','-u',str(step),name,'--',*argv],capture_output=True)
 print(name,'passed' if p.returncode==0 else p.stdout.decode('utf-8',errors='replace')+p.stderr.decode('utf-8',errors='replace'),flush=True)
 if p.returncode:raise SystemExit(p.returncode)
