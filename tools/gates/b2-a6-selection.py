"""受控测量模式入口：双开启必须失败，不静默选择B1或A6。"""
import json
import os
import subprocess
import sys


def mode(environment):
    flags = [environment.get('XIAO_B1_F10', '0'), environment.get('XIAO_B2_A_PERF', '0')]
    if any(value not in {'0', '1'} for value in flags):
        raise ValueError('measurement flags must be 0 or 1')
    if flags == ['1', '1']:
        raise ValueError('b1_f10 and b2_a_perf are mutually exclusive')
    return 'b1' if flags[0] == '1' else 'b2-a6' if flags[1] == '1' else 'off'


def self_test():
    cases = [('0', '0', 0, 'off'), ('1', '0', 0, 'b1'), ('0', '1', 0, 'b2-a6'),
             ('1', '1', 2, None), ('true', '0', 2, None), ('0', '', 2, None)]
    for b1, a6, code, expected in cases:
        environment = os.environ.copy()
        environment.update(XIAO_B1_F10=b1, XIAO_B2_A_PERF=a6)
        result = subprocess.run([sys.executable, __file__], env=environment, capture_output=True)
        assert result.returncode == code, result.stderr.decode()
        if expected:
            assert json.loads(result.stdout)['measurement_mode'] == expected
        elif b1 == a6 == '1':
            assert result.stderr.decode().strip() == 'b1_f10 and b2_a_perf are mutually exclusive'
    print(json.dumps({'passed': True, 'cases': len(cases), 'scope': 'actual measurement-selection entry'}))


if __name__ == '__main__':
    if sys.argv[1:] == ['--self-test']:
        self_test()
    else:
        try:
            print(json.dumps({'measurement_mode': mode(os.environ)}))
        except ValueError as error:
            print(error, file=sys.stderr)
            raise SystemExit(2)
