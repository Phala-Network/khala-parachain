#!/bin/bash

SECRET="${SECRET:-"extend split brush maximum nominee oblige merit modify latin never shiver slide"}"
N="${N:-"1"}"
ENDPOINT="${ENDPOINT:-"http://127.0.0.1:9933"}"

function get_pubkey {
  tmp=tmp.key
  ./polkadot "$@" > "$tmp"
  awk '/Public key \(hex\): +(\w+?)/{print $4}' "$tmp"
  rm tmp.key
}

function insert_key {
  type="$1"
  suri="$2"

  if [ "$type" = 'gran' ]; then
    pubkey=$(get_pubkey key inspect --scheme ed25519 "$suri")
  elif [ "$type" = 'beef' ]; then
    pubkey=$(get_pubkey key inspect --scheme ecdsa "$suri")
  else
    pubkey=$(get_pubkey key inspect --scheme sr25519 "$suri")
  fi

  curl "$ENDPOINT" -H "Content-Type:application/json;charset=utf-8" -d \
    """{
      \"jsonrpc\":\"2.0\",
      \"id\":1,
      \"method\":\"author_insertKey\",
      \"params\": [
        \"${type}\",
        \"${suri}\",
        \"${pubkey}\"
      ]
    }"""
}

insert_key babe "${SECRET}//${N}//babe"
insert_key gran "${SECRET}//${N}//grandpa"
insert_key imon "${SECRET}//${N}//im_online"
insert_key para "${SECRET}//${N}//para_validator"
insert_key asgn "${SECRET}//${N}//para_assignment"
insert_key audi "${SECRET}//${N}//authority_discovery"
insert_key beef "${SECRET}//${N}//beefy"
