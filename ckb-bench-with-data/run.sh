bash script/run.sh setup
bash script/run.sh run
bash script/run.sh clean_job
python script/gen_report.py
report=`cat demo.md`
export GITHUB_TOKEN=${GITHUB_TOKEN}
bash script/ok.sh add_comment nervosnetwork/ckb-integration-test 116 "$report"
