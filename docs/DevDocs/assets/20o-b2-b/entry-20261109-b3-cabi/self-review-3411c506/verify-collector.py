"""在复制归档重生成收口，核对与旧实际字节一致，不改写原记录。"""
from contextlib import redirect_stdout
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import sys
import tempfile

sys.dont_write_bytecode = True
HERE=Path(__file__).resolve().parent
ROOT=HERE.parent


def main():
    spec=importlib.util.spec_from_file_location('collector',ROOT/'collect.py')
    module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    with tempfile.TemporaryDirectory(prefix='b3-collector-review-') as temporary:
        target=Path(temporary)/'evidence'
        shutil.copytree(ROOT,target,ignore=shutil.ignore_patterns('__pycache__'))
        names=['closeout.json','code-identity.json']
        before={name:hashlib.sha256((target/name).read_bytes()).hexdigest() for name in names}
        module.ROOT=target
        buffer=io.StringIO()
        with redirect_stdout(buffer):module.main(False)
        after={name:hashlib.sha256((target/name).read_bytes()).hexdigest() for name in names}
        assert before==after,'日志重算结果与历史收口字节不同'
        result={'passed':True,'copied_directory':str(target),'output':buffer.getvalue().strip(),
                'originals_overwritten':False,'before':before,'after':after,
                'collector_sha256':hashlib.sha256((ROOT/'collect.py').read_bytes()).hexdigest()}
    destination=HERE/'collector-result.json'
    if destination.exists():raise SystemExit('拒绝覆盖旧复核结果')
    destination.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf8',newline='\n')
    print(json.dumps(result,ensure_ascii=True))


if __name__=='__main__':main()
