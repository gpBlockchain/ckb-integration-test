bash script/run.sh setup
bash script/run.sh run 98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946.1w.1024w.cell
bash script/run.sh bench

pip install qiniu
python script/gen_report.py
report=`cat demo.md`
export GITHUB_TOKEN=${GITHUB_TOKEN}
bash script/ok.sh add_comment nervosnetwork/ckb 2372 "$report"

python wkr.py
report=`cat wkr.md`
export GITHUB_TOKEN=${GITHUB_TOKEN}
bash script/ok.sh add_comment cryptape/acceptance-internal 781 "$report"
