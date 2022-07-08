#!/usr/bin/env node
import 'source-map-support/register';
import * as cdk from 'aws-cdk-lib';
import { DynamodbStreamToGrpcStack } from '../lib/dynamodb-stream-to-grpc-stack';

const app = new cdk.App();
new DynamodbStreamToGrpcStack(app, 'DynamodbStreamToGrpcStack');
