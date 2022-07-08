import { Construct } from 'constructs';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as elbv2 from 'aws-cdk-lib/aws-elasticloadbalancingv2';
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as acm from 'aws-cdk-lib/aws-certificatemanager';
import { DockerImageAsset } from 'aws-cdk-lib/aws-ecr-assets';

export interface GrpcServerProps {
  vpc: ec2.Vpc;
  cert: acm.ICertificate;
  cluster: ecs.Cluster;
  container: string;
  dockerfile: string;
  environment: { [key: string]: string };
}

export class GrpcServer extends Construct {
  public readonly alb: elbv2.ApplicationLoadBalancer;
  public readonly listener: elbv2.ApplicationListener;
  public readonly targetGroup :elbv2.ApplicationTargetGroup;
  public readonly taskDefinition: ecs.FargateTaskDefinition;
  public readonly container: ecs.ContainerDefinition;
  public readonly service: ecs.FargateService;

  constructor(scope: Construct, id: string, props: GrpcServerProps) {
    super(scope, id);

    const alb = new elbv2.ApplicationLoadBalancer(this, `${id}_GrpcAlb`, {
      vpc: props.vpc,
      internetFacing: true,
    });

    const listener = alb.addListener(`${id}_GrpcListener`, {
      protocol: elbv2.ApplicationProtocol.HTTPS,
      port: 50051,
      open: true,
      certificates: [props.cert],
    });

    const targetGroup = new elbv2.ApplicationTargetGroup(this, `${id}_TargetGroup`, {
      targetType: elbv2.TargetType.IP,
      port: 50051,
      protocol: elbv2.ApplicationProtocol.HTTP,
      protocolVersion: elbv2.ApplicationProtocolVersion.GRPC,
      healthCheck: {
        enabled: true,
        healthyGrpcCodes: '0-99',
      },
      vpc: props.vpc,
    });

    const taskDefinition = new ecs.FargateTaskDefinition(this, `${id}_TaskDefinition`);
    const container = taskDefinition.addContainer(`${id}_GrpcServer`, {
      image: ecs.ContainerImage.fromDockerImageAsset(new DockerImageAsset(this, `${id}_GrpcServerImage`, {
        directory: props.container,
        file: props.dockerfile,
      })),
      logging: ecs.LogDrivers.awsLogs({ streamPrefix: `${id}_GrpcServerLog` }),
      environment: props.environment,
    });

    container.addPortMappings({
      containerPort: 50051,
    });

    const service = new ecs.FargateService(this, `${id}_GrpcService`, {
      cluster: props.cluster,
      taskDefinition,
    });

    listener.addTargetGroups(`${id}_Targets`, {
      targetGroups: [targetGroup],
    });

    targetGroup.addTarget(service);

    this.alb = alb;
    this.listener = listener;
    this.targetGroup = targetGroup;
    this.taskDefinition = taskDefinition;
    this.container = container;
    this.service = service;
  }
}
