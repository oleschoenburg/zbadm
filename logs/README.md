## Usage

Given several log files in a directory, parse all of them and print them out as linebreak separated JSON:

```shell
$ zbadm-logs ~/my-log-files/ > logs.json
51743 entries in a day from 2023-06-01 12:36:25.582 to 2023-06-02 14:12:53.114, averaging 0.49 entries per second
Brokers: {0, 1, 2}
Partitions: {1, 2, 3}
```

The resulting logs look like this:
```json
{"timestamp":"2023-06-01T16:36:30.766","lines":[699,700],"file":"my-log-file-1.txt","actor":{"broker":1,"name":"ZeebePartition","partition":1},"thread":"Broker-1-zb-actors-2","level":"INFO","logger":"io.camunda.zeebe.broker.system","message":"Transition to FOLLOWER on term 54 requested."}
{"timestamp":"2023-06-01T16:36:30.767","lines":[701,702],"file":"another-log-file.txt","actor":{"broker":1,"name":"ZeebePartition","partition":3},"thread":"Broker-1-zb-actors-4","level":"INFO","logger":"io.camunda.zeebe.broker.system","message":"Startup LogDeletionService"}
```

Any further processing can be done with `jq` and similar:

```shell
$ jq '.actor.partition == 1 and level == "WARN" < logs.json'
```