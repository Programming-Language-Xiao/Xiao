"""复核逐进程字段与归档复跑隔离；只改临时副本，不回写测量原件。"""
import argparse,hashlib,json,subprocess,sys,tempfile,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parent
ARCHIVE=ROOT.parent/'entry-20261107'
parser=argparse.ArgumentParser()
parser.add_argument('--output',type=Path,default=ROOT/'process-controls')
output=parser.parse_args().output;output.mkdir()
rows=[]
def invoke(name,argv,error):
    result=subprocess.run(argv,capture_output=True)
    (output/(name+'.stdout.log')).write_bytes(result.stdout);(output/(name+'.stderr.log')).write_bytes(result.stderr)
    actual=result.stderr.decode('utf-8').strip()
    passed=result.returncode==1 and not result.stdout and actual==error
    row={'case':name,'passed':passed,'argv':argv,'exit_code':result.returncode,'expected_error':error,'actual_error':actual,'stdout':name+'.stdout.log','stderr':name+'.stderr.log'}
    rows.append(row);print(json.dumps({'case':name,'passed':passed},ensure_ascii=False),flush=True)
    assert passed,row
with tempfile.TemporaryDirectory(prefix='a6-process-controls-') as temporary:
    directory=Path(temporary)
    with zipfile.ZipFile(ARCHIVE/'performance-archives/local-native-decimal.zip') as stream:stream.extractall(directory)
    report=json.loads((directory/'report.json').read_text())
    batch=report['records'][0]['batches'][0]
    relative=batch['stdout'].replace('\\','/').split('-artifacts/',1)[1]
    process=directory/'artifacts'/relative.replace('.stdout.jsonl','.process.json')
    original=process.read_bytes()
    for name,mutate,error in [
        ('missing-process-executable',lambda r:r.pop('executable'),'原始进程执行文件或参数未绑定实际产物'),
        ('missing-process-argv',lambda r:r.pop('argv'),'原始进程执行文件或参数未绑定实际产物'),
        ('unexpected-process-argv',lambda r:r.update(argv=['--old-cache']),'原始进程执行文件或参数未绑定实际产物'),
        ('mismatched-start-time',lambda r:r.update(started_unix_nanoseconds='1'),'原始进程启动时间不符'),
        ('mismatched-output-path',lambda r:r.update(stdout=r['stdout']+'.other'),'原始进程输出路径或规模关联不符')]:
        record=json.loads(original);mutate(record);process.write_text(json.dumps(record),encoding='utf-8')
        invoke(name,[sys.executable,'-X','utf8',str(ARCHIVE/'verify-performance.py'),'--report',str(directory/'report.json'),'--artifacts',str(directory/'artifacts'),'--require-full'],error)
        process.write_bytes(original)
    with zipfile.ZipFile(ARCHIVE/'performance-archives/local-vm-decimal.zip') as stream:stream.extractall(directory/'vm')
    result=subprocess.run([sys.executable,'-X','utf8',str(ARCHIVE/'verify-negative-cases.py'),'--report',str(directory/'vm/report.json'),'--artifacts',str(directory/'vm/artifacts'),'--output',str(output/'report-negative-cases.json')],capture_output=True)
    (output/'report-negative-cases.stdout.log').write_bytes(result.stdout);(output/'report-negative-cases.stderr.log').write_bytes(result.stderr)
    assert result.returncode==0,result.stderr.decode('utf-8')
    print(result.stdout.decode('utf-8').strip(),flush=True)
forbidden=ARCHIVE/'forbidden-review-output'
assert not forbidden.exists()
manifest_before=hashlib.sha256((ARCHIVE/'evidence.json').read_bytes()).hexdigest()
invoke('in-archive-output',[sys.executable,'-X','utf8',str(ARCHIVE/'validate-audit.py'),'--output',str(forbidden)],'反例输出必须位于原归档之外，避免改变历史清单')
assert not forbidden.exists() and hashlib.sha256((ARCHIVE/'evidence.json').read_bytes()).hexdigest()==manifest_before
(output/'cases.json').write_text(json.dumps({'passed':True,'cases':rows,'report_negative_cases':len(json.loads((output/'report-negative-cases.json').read_text())['cases']),'archive_manifest_unchanged':True},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
