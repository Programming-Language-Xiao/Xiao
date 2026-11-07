"""保存GitHub运行/每次尝试/实际作业日志和artifact原件，不以聚合ID代替日志来源。"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess

REPO='repos/Programming-Language-Xiao/Xiao'


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--run',required=True,type=int)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--download-artifacts',action='store_true')
    parser.add_argument('--git-proxy',action='store_true')
    args=parser.parse_args()
    args.output.mkdir(parents=True,exist_ok=False)
    calls=[]
    environment=os.environ.copy()
    if args.git_proxy:
        proxy=subprocess.check_output(['cmd','/c','git','config','--get','http.proxy'],text=True).strip()
        environment.update(HTTP_PROXY=proxy,HTTPS_PROXY=proxy)
    def get(endpoint,name):
        argv=['gh','api',REPO+'/'+endpoint]
        started=datetime.datetime.now(datetime.timezone.utc).isoformat()
        for attempt in range(1,4):
            try:
                result=subprocess.run(argv,env=environment,capture_output=True,timeout=55)
                returncode=result.returncode
                stdout=result.stdout
                stderr=result.stderr
            except subprocess.TimeoutExpired as error:
                returncode=124; stdout=error.stdout or b''; stderr=error.stderr or b'GitHub API subprocess timeout'
            actual=name if returncode==0 else name+f'.request-{attempt}.stderr'
            raw=stdout if returncode==0 else stderr
            (args.output/actual).write_bytes(raw)
            calls.append({'argv':argv,'started_utc':started,'file':actual,'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest(),'exit_code':returncode,'network':'existing git http.proxy; child only' if args.git_proxy else 'inherited environment'})
            (args.output/'commands.json').write_text(json.dumps({'run':args.run,'calls':calls},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
            if returncode==0: return stdout
        raise SystemExit(stderr.decode('utf-8',errors='replace'))
    run=json.loads(get(f'actions/runs/{args.run}','run.json'))
    if run['status']!='completed': raise SystemExit('运行尚未结束，不能封存最终作业来源')
    jobs=[]
    for attempt in range(1,run['run_attempt']+1):
        data=json.loads(get(f'actions/runs/{args.run}/attempts/{attempt}/jobs?per_page=100',f'attempt-{attempt}-jobs.json'))
        assert data['total_count']==len(data['jobs']), '作业分页不完整'
        jobs.extend(data['jobs'])
    unique={job['id']:job for job in jobs}
    (args.output/'jobs.json').write_text(json.dumps({'jobs':list(unique.values())},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    # 每个原始作业ID都保存自己的日志；保留执行的新映射若已过期可用原始ID复核。
    for job in unique.values():
        if job['conclusion']!='skipped':
            try:
                get(f'actions/jobs/{job["id"]}/logs',f'job-{job["id"]}.log')
            except SystemExit:
                # 最新映射ID在重跑时可能没有独立日志；原始作业仍必须成功保存。
                if run['run_attempt']==1: raise
    artifacts=json.loads(get(f'actions/runs/{args.run}/artifacts?per_page=100','artifacts.json'))
    assert artifacts['total_count']==len(artifacts['artifacts']), '产物分页不完整'
    if args.download_artifacts:
        for item in artifacts['artifacts']:
            if item['name'].startswith('b2-a6-'):
                raw=get(f'actions/artifacts/{item["id"]}/zip',f'artifact-{item["id"]}.zip')
                assert item.get('digest') in (None,'sha256:'+hashlib.sha256(raw).hexdigest()), '远端产物摘要不符'
    (args.output/'commands.json').write_text(json.dumps({'run':args.run,'collected_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'calls':calls},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    print(json.dumps({'run':args.run,'attempts':run['run_attempt'],'conclusion':run['conclusion'],'jobs':len(unique),'artifacts':len(artifacts['artifacts'])},ensure_ascii=False))

if __name__=='__main__':
    main()
