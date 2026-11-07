from pathlib import Path
import json,subprocess,sys
step=Path(__file__).with_name('run-step.py');source=step.parents[1]/'source'
commands=[('entry-a0-audit','docs/DevDocs/assets/20o-b2-a0/entry-20261105/verify-evidence.py'),('entry-a1-audit','docs/DevDocs/assets/20o-b2-a1/entry-20261106/audit.py'),('entry-a2-a3-audit','docs/DevDocs/assets/20o-b2-a2-a3/entry-20261106/audit.py'),('entry-a4-audit','docs/DevDocs/assets/20o-b2-a4/entry-20261106/audit.py'),('entry-a5-audit','docs/DevDocs/assets/20o-b2-a5/entry-20261106/audit.py'),('entry-a6-audit','docs/DevDocs/assets/20o-b2-a6/entry-20261107/audit.py'),('entry-a6-review','docs/DevDocs/assets/20o-b2-a6/self-review-20261107/verify-review.py')]
for name,script in commands:
 p=subprocess.run([sys.executable,'-X','utf8','-u',str(step),name,'--',sys.executable,'-X','utf8',script],capture_output=True)
 print(name,'passed' if p.returncode==0 else p.stdout.decode('utf-8',errors='replace'),flush=True)
 if p.returncode:raise SystemExit(p.returncode)
