#!/bin/bash -e

protodir=proto
sources=(etcdserver/etcdserverpb/rpc.proto mvcc/mvccpb/kv.proto auth/authpb/auth.proto)
branch="${1:-master}"

mkdir -p $protodir
# TODO: lock API?

sedscript='
/gogoproto/d;
/annotations.proto/d;
/^import/s|".*/|"|;
/^\s*option (google\.api\.http) = {/ {
  :a;
  N;
  /};/!ba;
  d
}
'
echo $s

for source in "${sources[@]}"; do
  path="$protodir/$(basename $source)"
  mkdir -p $(dirname "$path")
  curl -f "https://raw.githubusercontent.com/etcd-io/etcd/$branch/$source" \
    | sed -e "$sedscript" > "$path"
done
