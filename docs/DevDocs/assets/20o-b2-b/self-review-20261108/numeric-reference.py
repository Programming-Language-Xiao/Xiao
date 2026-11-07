"""以Python Decimal/libmpdec独立验算有效数字舍入；链接既有B2 rlib验证C别名。"""
from decimal import Decimal, localcontext
import decimal
import hashlib
import json
import os
from pathlib import Path
import random
import subprocess
import sys

root=Path(__file__).resolve().parent
original=root.parent/'entry-20261108-b2'
setup=json.loads((original/'environment.json').read_text(encoding='utf-8'))
task=Path(json.loads((original/'bootstrap.json').read_text(encoding='utf-8'))['task_root'])
deps=task/'cargo-target/debug/deps'
rlib=max((p for p in deps.glob('libxiao_runtime-*.rlib')),key=lambda p:p.stat().st_mtime_ns)
abi=max((p for p in deps.glob('libxiao_runtime_abi-*.rlib')),key=lambda p:p.stat().st_mtime_ns)
output=task/'self-review';output.mkdir(exist_ok=True)
exe=output/'numeric-probe.exe'
env={key:value for key,value in os.environ.items() if not key.startswith('RUST_BIGDECIMAL_')}
env.update(setup['msvc_whitelist']);env.update(setup['overrides'])
argv=['rustup','run','1.96.0','rustc','--edition','2024',str(root/'numeric-probe.rs'),
      '-L','dependency='+str(deps),'--extern','xiao_runtime='+str(rlib),'--extern','xiao_runtime_abi='+str(abi),'-o',str(exe)]
compiled=subprocess.run(argv,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,encoding='utf-8')
if compiled.returncode:raise SystemExit(compiled.stdout)
modes={'half_even':decimal.ROUND_HALF_EVEN,'half_up':decimal.ROUND_HALF_UP,'half_down':decimal.ROUND_HALF_DOWN,
       'toward_zero':decimal.ROUND_DOWN,'away_from_zero':decimal.ROUND_UP,'floor':decimal.ROUND_FLOOR,'ceiling':decimal.ROUND_CEILING}
rng=random.Random(20261108)
inputs=[]
for _ in range(320):
    left=f'{rng.choice([-1,1])*rng.randrange(1,10**18)}e{rng.randrange(-200,201)}'
    right=f'{rng.choice([-1,1])*rng.randrange(1,10**18)}e{rng.randrange(-200,201)}'
    precision=rng.randrange(1,33)
    for mode in modes:inputs.append((left,right,precision,mode))
for left,right in [('2.5','1'),('3.5','1'),('-3.5','1'),('2.5000001','1'),('2.4999999','1'),('9.999','1'),('1','8'),('1','-8')]:
    for mode in modes:inputs.append((left,right,1,mode))
inputs.extend([('1','2',32768,'half_even'),('2','3',32768,'half_even'),('1e-100000','1',32768,'half_even'),('1e100000','1',32768,'half_even')])
payload=''.join(f'{left}\t{right}\t{precision}\t{mode}\n' for left,right,precision,mode in inputs)
process=subprocess.run([str(exe)],env=env,input=payload,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8')
rows=process.stdout.splitlines();assert process.returncode==0,process.stderr
assert rows.pop(0)=='alias_checks=3'
assert len(rows)==len(inputs)
mismatches=[]
for args,actual in zip(inputs,rows):
    left,right,precision,mode=args
    with localcontext() as context:
        context.prec=precision;context.rounding=modes[mode]
        expected=Decimal(left)/Decimal(right)
    if actual.startswith('ERR:') or Decimal(actual)!=expected:mismatches.append({'args':args,'expected':str(expected),'actual':actual})
report={'passed':not mismatches,'numeric_cases':len(inputs),'c_abi_alias_cases':3,'mismatches':mismatches,
        'reference':{'python':sys.version,'libmpdec':decimal.__libmpdec_version__,'seed':20261108},
        'compile_argv':argv,'runtime_rlib':{'path':str(rlib),'sha256':hashlib.sha256(rlib.read_bytes()).hexdigest()},
        'abi_rlib':{'path':str(abi),'sha256':hashlib.sha256(abi.read_bytes()).hexdigest()},
        'probe_sha256':hashlib.sha256((root/'numeric-probe.rs').read_bytes()).hexdigest(),
        'exe_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'input_sha256':hashlib.sha256(payload.encode()).hexdigest(),
        'output_sha256':hashlib.sha256(process.stdout.encode()).hexdigest()}
(root/'numeric-result.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps({key:report[key] for key in ['passed','numeric_cases','c_abi_alias_cases','mismatches']},ensure_ascii=True))
raise SystemExit(0 if report['passed'] else 1)
