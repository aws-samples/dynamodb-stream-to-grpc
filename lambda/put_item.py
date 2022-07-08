import boto3
import os
import uuid
from random import randint

dynamodb = boto3.resource('dynamodb')
table = dynamodb.Table(os.environ['TABLE'])

def handler(_event, _context):
    table.put_item(
        Item={
            'id': str(uuid.uuid4()),
            'value': randint(0, 100),
        }
    )
