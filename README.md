# Cached DNS Resolver

Let's say you are writing a proxy and you want to check that the URLs provided do not resolve to an internal endpoint or service. You might write something like this (in vague pseudocode):

```rust
let response = query_dns_server(url);

if validate_dns_result(response).is_err() {
	return Err("DNS response contained some problems");
}

make_request(url);
```

This makes sense, we first query the result from the DNS server, then do some processing on it and finally make the request when we are happy it is safe.

## The Problem

However, this is vulnerable to a DNS rebinding attack (a form of Time of Check -> Time of Use attack) where an attacker can cause the DNS resolution to change whilst you run `validate_dns_result`. This means the initial DNS search might return an external IP, but the actual request client will then run the DNS query again and use whatever it gets back without re-validating.

This crate solves this issue by caching the original DNS result and forcing the request client to re-use the value when it goes to make the request. The initial query will use the default DNS resolver (`getaddrinfo`) and then subsequent queries will just use the cache.

## What happens if the DNS result expires?

Then the request will fail. A new instance of the cached resolver should be created for each request you make in this fashion.
