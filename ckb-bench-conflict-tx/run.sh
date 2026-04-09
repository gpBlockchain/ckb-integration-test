pip install paramiko pyyaml -q 2>/dev/null || true
pip install qiniu
bash script/benchmark.sh setup
bash script/benchmark.sh run
bash script/benchmark.sh clean
