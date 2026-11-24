import pathlib
import re
import sys
text = pathlib.Path(sys.argv[1]).read_text()
assert len(re.findall(r'^running 1 test$', text, re.M)) == 1, 'expected one actual test'
assert re.search(r'^test ' + re.escape(sys.argv[2]) + r' \.\.\. ok$', text, re.M), 'exact test did not pass'
assert re.search(r'^test result: ok\. 1 passed; 0 failed; 0 ignored;', text, re.M), 'expected one passed, none ignored'
assert not re.search(r'^running 0 tests$', text, re.M), 'zero matches is not success'
print('PASS: retained-Seed parent characterization ran exactly one matching test')
