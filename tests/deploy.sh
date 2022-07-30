kubectl apply -f tests/data
docker build . -t maheshrayas/kube-saver-local:latest
kind load docker-image maheshrayas/kube-saver-local:latest
kubectl apply -k ./tests/controller
