* Deploy the kube-saver as Deployment spec
* Deploy the CRD for kube-upscaler
* Downscaler loop will be continously running
* Scenario 1: Downscaler has scaled down all the deployments
    * kubectl apply kind: Upscaler
    * code will read the labels that needs to be upscaled, have functionality to enter replicas.
    * Dont do any action on delete
