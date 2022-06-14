# url-shorter

A simple, low level url shorting service written with [`tiny_http`](https://github.com/tiny-http/tiny-http).\
The service can scale to around 6 cpu cores under heavy load.\
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

```text
Operating System: Manjaro Linux
Kernel Version: 5.15.41-1-MANJARO (64-bit)
Processors: 12 × AMD Ryzen 5 3600 6-Core Processor
Memory: 31.3 GiB of RAM
```

```bash
# Adding, heavily depends on I/O speed, so results may vary.
$ wrk -t1 -c4 -d30s -s scripts/add.lua --latency http://localhost:8000
Running 30s test @ http://localhost:8000
  1 threads and 4 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    73.31us  202.59us  14.42ms   99.78%
    Req/Sec    57.67k     5.65k   76.34k    78.41%
  Latency Distribution
     50%   68.00us
     75%   74.00us
     90%   80.00us
     99%   92.00us
  1726400 requests in 30.10s, 319.40MB read
Requests/sec:  57356.90
Transfer/sec:     10.61MB

# Querying.
$ wrk -t4 -c32 -d5m -s scripts/query.lua --latency http://localhost:8000
Running 5m test @ http://localhost:8000
  4 threads and 32 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   253.67us    0.98ms  24.97ms   95.57%
    Req/Sec    85.21k     7.16k  105.27k    71.77%
  Latency Distribution
     50%   71.00us
     75%   84.00us
     90%  118.00us
     99%    5.24ms
  101751428 requests in 5.00m, 13.45GB read
Requests/sec: 339080.72
Transfer/sec:     45.88MB

# Removing, skipped empty url check.
$ wrk -t1 -c4 -d30s -s scripts/remove.lua --latency http://localhost:8000/1103N-502e4050-b291-43cd-95e9-f93919faa41d
Running 30s test @ http://localhost:8000/1103N-502e4050-b291-43cd-95e9-f93919faa41d
  1 threads and 4 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    33.00us   31.37us   3.37ms   99.74%
    Req/Sec   119.44k     4.59k  133.35k    76.74%
  Latency Distribution
     50%   32.00us
     75%   34.00us
     90%   37.00us
     99%   46.00us
  3576800 requests in 30.10s, 484.38MB read
Requests/sec: 118831.54
Transfer/sec:     16.09MB
```
