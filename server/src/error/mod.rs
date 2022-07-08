use aws_sdk_dynamodb::error::*;
use aws_sdk_kinesis::error::*;
use aws_sdk_kinesis::types::SdkError;
use std::fmt;

#[derive(Debug)]
pub struct PollKinesisError {
    pub message: String,
}

impl fmt::Display for PollKinesisError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl From<SdkError<ListShardsError>> for PollKinesisError {
    fn from(_: SdkError<ListShardsError>) -> Self {
        PollKinesisError {
            message: "failed to get shards list".to_owned(),
        }
    }
}

impl From<SdkError<GetShardIteratorError>> for PollKinesisError {
    fn from(_: SdkError<GetShardIteratorError>) -> Self {
        PollKinesisError {
            message: "failed to get shard iterator".to_owned(),
        }
    }
}

impl From<SdkError<GetRecordsError>> for PollKinesisError {
    fn from(_: SdkError<GetRecordsError>) -> Self {
        PollKinesisError {
            message: "failed to get records".to_owned(),
        }
    }
}

impl From<QueryDynamoDbError> for PollKinesisError {
    fn from(q: QueryDynamoDbError) -> Self {
        PollKinesisError { message: q.message }
    }
}

#[derive(Debug)]
pub struct QueryDynamoDbError {
    pub message: String,
}

impl fmt::Display for QueryDynamoDbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl From<SdkError<QueryError>> for QueryDynamoDbError {
    fn from(_: SdkError<QueryError>) -> Self {
        QueryDynamoDbError {
            message: "query error".to_owned(),
        }
    }
}
