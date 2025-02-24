pip install qiniu
pip install discord
bash script/benchmark.sh setup
bash script/benchmark.sh run
bash script/benchmark.sh clean
python script/gen_report.py
report=`cat demo.md`
export GITHUB_TOKEN=${GITHUB_TOKEN}
bash script/ok.sh add_comment nervosnetwork/ckb 2372 "$report"
