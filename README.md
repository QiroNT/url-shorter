# url-shorter

A simple, low level url shorting service written with [`tiny_http`](https://github.com/tiny-http/tiny-http).\
The service is single threaded and uses at most 2 cpu cores under normal circumstances,
since multi-threading via a `RwLock` on `data` makes processing speed ~30% slower under the same cpu usage.\
**It's just a little side project, data integrity is not guaranteed!**

## API

### GET /\[id\]

Get a redirect by `[id]`.

<details>
<summary>302 Found</summary><br>

If the id is valid and the redirect exists,\
the server will respond as follows:

```http
HTTP/1.1 302 Found
Location: https://example.com/
Content-Length: 0

```

<br></details>

<details>
<summary>400 Invalid id</summary><br>

When passing an id with `[^0-9a-zA-Z]` in it, or the id decoded is larger than <code>2<sup>64</sup> − 1</code>,\
the server will respond as follows:

```http
HTTP/1.1 400 Bad Request
Content-Type: text/plain; charset=utf-8
Content-Length: 10

invalid-id
```

<br></details>

<details>
<summary>404 Not found</summary><br>

When passing an id that doesn't yet have a corresponding redirect or it has been deleted,\
the server will respond as follows:

```http
HTTP/1.1 404 Not Found
Content-Type: text/plain; charset=utf-8
Content-Length: 9

not-found
```

<br></details>

### POST /\[url\]

Add a new redirect to `[url]`.

<details>
<summary>200 Success</summary><br>

If the provided url is valid,\
the server will respond as follows:

```http
HTTP/1.1 200 OK
Content-Type: application/json
Content-Length: 60

{"id":"yc5c","token":"ee7d345c-d526-4b4e-96fb-ec770197335d"}
```

Where `id` is the id used to access the redirect, and `token` is the token used to remove the redirect.

Note: Every successful request will cause a file write.

<br></details>

<details>
<summary>400 Invalid URL</summary><br>

If the URL can't be parsed by `Url::parse`,\
the server will respond as follows:

```http
HTTP/1.1 400 Bad Request
Content-Type: text/plain; charset=utf-8
Content-Length: 22

invalid-url:empty-host
```

Where the reason behind the colon is one of the following:

- `empty-host`
- `idna-error`
- `invalid-port`
- `invalid-ipv4`
- `invalid-ipv6`
- `invalid-domain`
- `relative-url-without-base`
- `relative-url-with-cannot-be-a-base-base`
- `set-host-on-cannot-be-a-base-url`
- `overflow`
- `unknown`

<br></details>

<details>
<summary>500 Write error</summary><br>

If the server fails to write the redirect to `data.txt`,\
the server will respond as follows:

```http
HTTP/1.1 500 Internal Server Error
Content-Type: text/plain; charset=utf-8
Content-Length: 11

write-error
```

<br></details>

### DELETE /\[id\]-\[token\]

Remove a redirect by `[id]` with `[token]`.

<details>
<summary>200 Success</summary><br>

If the token is a valid UUID and the redirect exists,\
the server will respond as follows:

```http
HTTP/1.1 200 OK
Content-Type: text/plain; charset=utf-8
Content-Length: 20

https://example.com/
```

Where the response body is the url of the redirect.

Note: Every successful request will cause a file write to `prm.txt`, and a entire file rewrite to `data.txt` upon restart.

<br></details>

<details>
<summary>400 Invalid id</summary><br>

When passing an id with `[^0-9a-zA-Z]` in it, or the id decoded is larger than <code>2<sup>64</sup> − 1</code>,\
the server will respond as follows:

```http
HTTP/1.1 400 Bad Request
Content-Type: text/plain; charset=utf-8
Content-Length: 10

invalid-id
```

<br></details>

<details>
<summary>400 Invalid token</summary><br>

If the token is not a valid UUID or it's not the token of the redirect,\
the server will respond as follows:

```http
HTTP/1.1 400 Bad Request
Content-Type: text/plain; charset=utf-8
Content-Length: 13

invalid-token
```

<br></details>

<details>
<summary>400 Not found</summary><br>

If the id doesn't have a corresponding redirect exists,\
the server will respond as follows:

```http
HTTP/1.1 400 Bad Request
Content-Type: text/plain; charset=utf-8
Content-Length: 9

not-found
```

<br></details>

<details>
<summary>500 Write error</summary><br>

If the server fails to write the token to `prm.txt`,\
the server will respond as follows:

```http
HTTP/1.1 500 Internal Server Error
Content-Type: text/plain; charset=utf-8
Content-Length: 11

write-error
```

<br></details>

## Benchmark

```bash
# Adding, heavily depends on I/O speed, so results may vary.
$ wrk -t1 -c4 -d30s -s scripts/add.lua --latency http://localhost:8000
Running 30s test @ http://localhost:8000
  1 threads and 4 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   101.76us  206.94us  14.64ms   99.76%
    Req/Sec    41.02k     3.96k   48.73k    61.00%
  Latency Distribution
     50%   93.00us
     75%  109.00us
     90%  115.00us
     99%  131.00us
  1224585 requests in 30.00s, 226.52MB read
Requests/sec:  40819.18
Transfer/sec:      7.55MB

# Querying.
$ wrk -t2 -c6 -d5m -s scripts/query.lua --latency http://localhost:8000
Running 5m test @ http://localhost:8000
  2 threads and 6 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    28.88us   23.91us   9.72ms   99.52%
    Req/Sec    81.57k    16.57k  115.34k    57.36%
  Latency Distribution
     50%   28.00us
     75%   30.00us
     90%   33.00us
     99%   46.00us
  48720776 requests in 5.00m, 6.44GB read
Requests/sec: 162348.70
Transfer/sec:     21.97MB

# Removing, skipped empty url check.
$ wrk -t1 -c4 -d30s -s scripts/remove.lua --latency http://localhost:8000/Q8WB-832e26ce-f139-4bfc-a898-3557dd3830ee
Running 30s test @ http://localhost:8000/Q8WB-832e26ce-f139-4bfc-a898-3557dd3830ee
  1 threads and 4 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    66.76us   48.98us   6.18ms   99.84%
    Req/Sec    60.40k     5.08k   72.37k    64.00%
  Latency Distribution
     50%   63.00us
     75%   74.00us
     90%   79.00us
     99%   91.00us
  1802525 requests in 30.00s, 244.10MB read
Requests/sec:  60083.81
Transfer/sec:      8.14MB
```
