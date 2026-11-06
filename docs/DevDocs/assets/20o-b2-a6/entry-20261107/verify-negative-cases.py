"""对实际报告做有意破坏，必须以声明的原因拒绝，不把任意异常退出算通过。"""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--artifacts', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args=parser.parse_args()
    source=json.loads(args.report.read_text(encoding='utf-8'))
    verifier=Path(__file__).with_name('verify-performance.py')
    artifacts=args.artifacts or Path(source['artifacts'][0]['path']).parent
    cases=[]
    def case(name, mutate, reason):
        report=copy.deepcopy(source)
        mutate(report)
        cases.append((name,report,reason))
    case('abi-boundary', lambda r:r.update(runtime_abi=65555), '版本边界不符')
    case('missing-scale', lambda r:r['records'].pop(), '实际负载、优化或规模缺项/重复')
    case('missing-batch', lambda r:r['records'][0]['batches'].pop(), '失败、缺批或不可验证规模')
    case('missing-pid', lambda r:r['records'][0]['batches'][0].update(pid=0), '缺少真实PID或未完成校准')
    case('wrong-rss-unit', lambda r:r['records'][0]['batches'][0]['baseline'].update(unit='MB'), 'OS原始内存值或单位不符')
    case('false-median', lambda r:r['records'][0].update(median_seconds=r['records'][0]['median_seconds']*1.1), '五批归一化中位数/保守峰值重算不符')
    case('false-peak', lambda r:r['records'][0]['batches'][0].update(resident_increment_upper_bytes=r['records'][0]['batches'][0]['resident_increment_upper_bytes']+1), '单PID OS峰值增量重算不符')
    case('subset-as-full', lambda r:r.update(full_catalog=False), '子集不可当作全量A6验收')
    results=[]
    with tempfile.TemporaryDirectory(prefix='a6-audit-negative-') as temporary:
        for name,report,reason in cases:
            root=Path(temporary)
            path=root/(name+'.json')
            path.write_text(json.dumps(report),encoding='utf-8')
            result=subprocess.run([sys.executable,'-X','utf8',str(verifier),'--report',str(path),'--artifacts',str(artifacts),'--require-full'],capture_output=True)
            stdout=result.stdout.decode('utf-8'); stderr=result.stderr.decode('utf-8')
            passed=result.returncode==1 and stderr.strip()==reason and not stdout
            row={'case':name,'passed':passed,'exit_code':result.returncode,'expected_error':reason,'actual_stderr':stderr,'actual_stdout':stdout}
            results.append(row)
            if not passed:
                raise SystemExit(json.dumps(row,ensure_ascii=False))
    with args.output.open('x',encoding='utf-8',newline='\n') as stream:
        json.dump({'passed':True,'report':str(args.report.resolve()),'cases':results},stream,ensure_ascii=False,indent=2)
        stream.write('\n')
    print(json.dumps({'passed':True,'negative_cases':len(results)},ensure_ascii=False))

if __name__=='__main__':
    main()
