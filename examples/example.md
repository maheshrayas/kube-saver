# Examples

|Rule| Description|
|----|------------|
|[rules-downscale-all-namespaces-except](rules-all-ns.yaml)| * Scale down Deployments, Statefulset pods to 0 replicas, Set HPA 1, Disable CronJob in all the namespaces except kube-system & istio-system between time 7pm to 7AM on weekdays and entire weekend Sat & Sun|
|[rules-downscale-deployment](rules-all-deploy.yaml)| * Scale down Deployments with labels: "app:deployment-1" in all the namespaces between 7pm to 7AM on weekdays and entire weekend Sat & Sun|
|[rules-downscale-ss](rules-all-ss.yaml)| * Scale down Statefulset with labels: "app:ss-1" in all the namespaces between 7pm to 7AM on weekdays and entire weekend Sat & Sun|
|[rules-disable-cronjob](rules-all-cronjob.yaml)| * Disable Cronjob with labels: "app:cj-1" in all the namespaces between 7pm to 7AM on weekdays and entire weekend Sat & Sun|
|[rules-downscale-hpa](rules-all-hpa.yaml)| * Disable Hpa with labels: "app:hpa-1" in all the namespaces between 7pm to 7AM on weekdays and entire weekend Sat & Sun|
|[rules-downscale-individual-resources](rules-app-all.yaml)| * Downscale Deployment, SS and Cronjob(disable) between 7pm to 7AM on weekdays and entire weekend Sat & Sun|
|[rules-downtime-aftermidnight](rules-downtime-aftermidnight.yaml)| * If the resources are used in offset timezone and you want resouces to UP between 7AM-2AM(next day). This rule makes sure you have resouces scaledown from 2AM to 7AM and from Sat 2 AM to Monday 7AM.|
|[rules-up-all-weekdays](rules-up-all-weekdays.yaml)| * If the resources want to be up 24x5 (mon-fri). This rule will scale down resources from Saturday 12AM to Sunday 23:59.|

Refer to Unit [Testcase](../src/utils/time_check.rs) for more details and supported rules.
