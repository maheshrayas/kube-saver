<h1 align="center">
  <p align="center">Kube-depre</p>
</h1>
<div align="center">
  <a href="ttps://github.com/maheshrayas/kube-depre/actions/workflows/ci.yaml" alt="Build"><img src="https://github.com/maheshrayas/kube-saver/actions/workflows/ci.yaml/badge.svg" /></a>
   </div>

## Motivation

* Scale down cluster nodes by scaling down deployments during non-business hours and save $$, but if you need to scale back the resources eventhough its a scaledown, don't worry. You will have a Custom Resource which will scale up all resources and wont scale down until next scaledown period.

## Installation

* Install CRD

    ```bash
    kubectl apply -f k8s/crds/crd.yaml
    ```

* Configure your rules in k8s/rules.yaml

* Install kube-saver operator

    ```bash
    kubectl apply -k k8s/
    ```

## Note

This project is under development expect changes.
