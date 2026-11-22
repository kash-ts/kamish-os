import subprocess
import sys
import os

os.chdir(os.path.dirname(os.path.abspath(__file__)))
js_code = "import('vite').then(v => v.build()).then(() => process.exit(0)).catch(err => { console.error(err); process.exit(1); });"

res = subprocess.run(['node', '--input-type=module', '-e', js_code], capture_output=True, text=True, timeout=60)
print("EXIT:", res.returncode)
print("STDOUT:", res.stdout)
print("STDERR:", res.stderr)
sys.exit(res.returncode)
