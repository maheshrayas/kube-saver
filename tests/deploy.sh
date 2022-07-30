kubectl apply -f tests/data
docker build . -t maheshrayas/kube-saver-local:latest
kind load docker-image maheshrayas/kube-saver-local:latest --nodes chart-testing-control-plane --name chart-testing
kubectl apply -k ./tests/controller
