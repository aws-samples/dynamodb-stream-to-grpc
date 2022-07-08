import { Stack, StackProps } from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import * as ddb from 'aws-cdk-lib/aws-dynamodb';
import * as kds from 'aws-cdk-lib/aws-kinesis';
import * as lambda from '@aws-cdk/aws-lambda-python-alpha';
import { Runtime } from 'aws-cdk-lib/aws-lambda';
import { GrpcServer } from './constructs';
import * as path from 'path';

export class DynamodbStreamToGrpcStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    if (!process.env.CERT_ARN) {
      console.error('Environment variable CERT_ARN is not set!');
      console.error('```');
      console.error('export CERT_ARN=arn:aws:acm:<AWS_REGION>:<Account ID>:certificate/<Cert ID>');
      console.error('```');
      process.exit(1);
    }

    const stream = new kds.Stream(this, 'Stream');

    const table = new ddb.Table(this, 'Table', {
      partitionKey: { name: 'id', type: ddb.AttributeType.STRING },
      kinesisStream: stream,
    });

    const putItem = new lambda.PythonFunction(this, 'PutItem', {
      runtime: Runtime.PYTHON_3_9,
      entry: path.join(__dirname, '..', 'lambda'),
      index: 'put_item.py',
      handler: 'handler',
      environment: {
        TABLE: table.tableName,
      },
    });

    table.grantWriteData(putItem);

    const vpc = new ec2.Vpc(this, 'GrpcVpc');
    const cert = acm.Certificate.fromCertificateArn(this, 'Certificate', process.env.CERT_ARN);
    const cluster = new ecs.Cluster(this, 'GrpcCluster', { vpc });
    const server = new GrpcServer(this, 'GrpcServer', {
      vpc,
      cert,
      cluster,
      container: path.join(__dirname, '..'),
      dockerfile: 'Dockerfile',
      environment: {
        DYNAMODB_TABLE: table.tableName,
        KINESIS_STREAM: stream.streamName,
        RUST_BACKTRACE: '1',
      },
    });

    stream.grantRead(server.taskDefinition.taskRole);
    table.grantReadData(server.taskDefinition.taskRole);
  }
}
