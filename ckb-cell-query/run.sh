pip install paramiko pyyaml tomlkit -q 2>/dev/null || true
pip install qiniu
pip install discord
bash script/run.sh setup
bash script/run.sh run ${1:-1000w}
python script/gen_report.py
report=`cat demo.md`
export GITHUB_TOKEN=${GITHUB_TOKEN}
bash script/ok.sh add_comment nervosnetwork/acceptance-internal 1222 "$report"
