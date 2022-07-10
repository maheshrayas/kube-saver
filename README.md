<h1 align="center">
  <p align="center">Kube-Saver</p>
</h1>
<div align="center">
  <a href="ttps://github.com/maheshrayas/kube-depre/actions/workflows/ci.yaml" alt="Build"><img src="https://github.com/maheshrayas/kube-saver/actions/workflows/ci.yaml/badge.svg" /></a>
    <a href="https://codecov.io/gh/maheshrayas/kube-saver" alt="Lint"><img src="https://codecov.io/gh/maheshrayas/kube-saver/branch/main/graph/badge.svg?token=A44LLJERHG" /></a>

   </div>

## Motivation

* Scale down cluster nodes by scaling down Deployments, StatefulSet, CronJob, Hpa
during non-business hours and save $$, but if you need to scale back the resources eventhough its a scaledown, don't worry, You will have a Custom Resource which will scale up all resources and wont scale down until next scaledown period.

## Installation

* Install CRD

    ```bash
    kubectl apply -f k8s/crds/crd.yaml
    ```

* Configure your rules in [rules.yaml](k8s/rules.yaml)

* Install kube-saver operator

    ```bash
    kubectl apply -k k8s/
    ```

## Examples

```bash
rules:
  # scale down deployment with name go-app-deployment-2 when current time/day not in uptime
  - id: rules-downscale-deployments
    uptime: Mon-Fri 09:00-17:00 Australia/Sydney
    jmespath: "metadata.name == 'go-app-deployment-2'" 
    resource:
      - Deployment # type of resource
    replicas: 0 # either set the replicas:0 or any number during nonuptime 
  # scale down all deployment, statefulset, cronjob, hpa in namespace kuber when current time/day not in uptime, in this case hpa will be set to 1 as the desired replicas is set as 0
  - id: rules-downscale-all-deployments-in-namespace
    uptime: Mon-Fri 09:00-17:00 Australia/Sydney
    jmespath: "metadata.name == 'kuber'" 
    resource:
      - Namespace # type of resource
    replicas: 0 # either set the replicas:0 or any number during nonuptime 
  # scale down all statefulset in namespace kuber when current time/day not in uptime
  - id: rules-downscale-all-statefulset
    uptime: Mon-Fri 09:00-17:00 Australia/Sydney
    jmespath: "metadata.name == 'statefulset_name'" 
    resource:
      - StatefulSet # type of resource
    replicas: 0 # either set the replicas:0 or any number during nonuptime 
  # disable all cronjob with the labels current time/day not in uptime
  - id: rules-disable-all-cronjob
    uptime: Mon-Fri 09:00-17:00 Australia/Sydney
    jmespath: "metadata.labels.app == 'some_random_app'" 
    resource:
      - cronjob # type of resource
  # set minReplicas of HPA to 1
  - id: rules-set-hpa
    uptime: Mon-Fri 09:00-17:00 Australia/Sydney
    jmespath: "metadata.labels.app == 'some_random_app'" 
    resource:
      - hpa # type of resource
    replicas:1
  # set minReplicas of HPA to 1
  - id: combination-of-resources
    uptime: Mon-Fri 09:00-17:00 Australia/Sydney
    jmespath: "metadata.labels.app == 'some_random_app' || metadata.labels.service != 'some_random_service'" 
    resource:
      - Deployment # type of resource
      - Statefulset
    replicas:0

```

* Define uptime in [Olson timezone](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones) format

## How can I upscale resouce during the downtime.?

kube-saver will automatically upscale the resoures to orignial number of replicas when the current time falls between the `uptime` configured in rules.yaml. But if you want to manually scale up single deployment/statefulset or all the deployment & stateulset resources in Namespace, you have following options and it won't be scaled down until next day downtime.
Choose any of the option below:

* Configure [upscaled.yaml](./k8s/crds/upscaler.yaml) and 

  ```bash 
  kubectl apply -f ./k8s/crds/upscaler.yaml

  ```
  
  Or

* Redeploy your deployment.

  Or

* Edit the uptime in [rules.yaml](./k8s/rules.yaml) and redeploy the operator

    ```bash
    kubectl apply -k k8s/
    ```

## Tested

| Kubernetes Provider |  Tested |
|----------------------|--------|
| Google Kubernetes Engine |   ✅   |
| KIND(no autoscaler)                 |   ✅   |

## Note

This project is under development expect changes.
