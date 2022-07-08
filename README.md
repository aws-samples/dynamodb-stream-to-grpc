# DynamoDB stream to gRPC

## What is this project?

The goal of this project is to notify clients of DynamoDB changes.
DynamoDB changes are sent to a Kinesis Data Stream, which is polled from a container in ECS.
From the ECS, the client is notified by gRPC's server streaming feature.
It can be said that DynamoDB changes are converted to gRPC.
The ECS container is implemented in the Rust language.

## How can this project help you?
- Those who want to integrate DynamoDB and Kinesis Data Stream and notify clients of changes
- Those who want to stream from ECS with gRPC
- Those who want to run Rust containers on ECS
- Those who want to implement grpc in Rust
- Those who want to know how to configure a CDK project including Protocol Buffer

## Architecture
![](/imgs/dynamodb-stream-to-grpc.png)

## Deployment

First, you need to create a certificate to enable to connect gRPC with TLS.

If you don't have a domain, you can create a self-signed certificate for `*.<AWS_REGION>.elb.amazonaws.com`. **Use it only for the verification. It's not secure.** Replace `<AWS_REGION>` to wherever you want. (e.g. `us-west-1`)
```bash
openssl genrsa -out ./server.key 2048
openssl req -new -key ./server.key -out ./server.csr
# Common Name is `*.<AWS_REGION>.elb.amazonaws.com`
openssl x509 -in ./server.csr -days 365 -req -signkey ./server.key -out ./server.crt
```

Open [ACM](https://console.aws.amazon.com/acm/home#/certificates/list) and click "Import" at right top. `Certificate body` is a content of `server.crt`, `Certificate private key` is a content of `server.key`, and leave the `Certificate chain` blank. Then click "Next". After the creation, copy the ARN of the certificate and set it as an environment variable like this.
```bash
export CERT_ARN=arn:aws:acm:<AWS_REGION>:<Account ID>:certificate/<Cert ID>
```

If you have a domain, you can skip the creation but still need to set your certificate as an environment variable. Also you need to create an A record to your Route53 after the Application Load Balancer deployment in the following steps.

Next, install the npm modules by the command below.
```bash
npm install
```

Next, let's execute the deployment by CDK. If you are new to CDK, you need to bootstrap your account by the command below.
```bash
npx cdk bootstrap
```

Then, execute the command below for the deployment.
```bash
npx cdk deploy --require-approval never
```

## Verification

Install [grpcurl](https://github.com/fullstorydev/grpcurl) command. Execute the command below to subscribe gRPC. Replace `<YOUR_ALB_DOMAIN>` to the endpoint of your application load balancer. You can confirm it in [here](https://console.aws.amazon.com/ec2/v2/home#LoadBalancers:sort=loadBalancerName).Note the `-insecure` flag is required since we use self-signed certificate. If you use your public certificate, you can remove the flag.
```bash
grpcurl \
    -insecure \
    -import-path ./proto \
    -proto ddbstream.proto \
    <YOUR_ALB_DOMAIN>:50051 ddbstream.DdbStream/Subscribe
```

Next, create a new item in DynamoDB. We created a lambda function to do it easily. Open [Lambda function](https://console.aws.amazon.com/lambda/home#/functions), find a lambda function with "DynamodbStreamToGrpcStack" prefix, and hit the "Test" button in the "Test" tab. You should receive a new item in the `grpcurl` connection.

## Security

See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## License

This library is licensed under the MIT-0 License. See the LICENSE file.
