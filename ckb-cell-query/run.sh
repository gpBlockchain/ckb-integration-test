bash script/run.sh setup
bash script/run.sh run 98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946.1w.1024w.cell
python wkr.py
report=`cat demo.md`
export GITHUB_TOKEN=${GITHUB_TOKEN}
bash script/ok.sh add_comment nervosnetwork/ckb 2372 "$report"
