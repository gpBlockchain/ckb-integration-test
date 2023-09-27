pip install qiniu
bash script/run.sh setup
bash script/run.sh run 1000w
bash script/run.sh restart_ckb 1000w
bash script/run.sh clean_ckb_env
bash script/run.sh run 2000w
bash script/run.sh restart_ckb 2000w
bash script/run.sh clean_job
python script/gen_report.py
report=`cat demo.md`
report+=`cat restart_cost_time.md`
export GITHUB_TOKEN=${GITHUB_TOKEN}
bash script/ok.sh add_comment nervosnetwork/ckb 2372 "$report"
