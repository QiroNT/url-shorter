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
    Latency   140.18us    0.96ms  43.06ms   99.57%
    Req/Sec    39.43k     2.12k   43.70k    75.75%
  Latency Distribution
     50%   98.00us
     75%  108.00us
     90%  120.00us
     99%  306.00us
  1181097 requests in 30.10s, 218.47MB read
Requests/sec:  39240.14
Transfer/sec:      7.26MB

# Querying.
$ wrk -t2 -c6 -d5m -s scripts/query.lua --latency http://localhost:8000
Running 5m test @ http://localhost:8000
  2 threads and 6 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    39.62us   61.09us   6.69ms   99.39%
    Req/Sec    76.42k     4.93k   95.99k    74.34%
  Latency Distribution
     50%   36.00us
     75%   40.00us
     90%   46.00us
     99%   76.00us
  45647931 requests in 5.00m, 6.03GB read
Requests/sec: 152109.38
Transfer/sec:     20.58MB

# Removing, skipped empty url check.
$ wrk -t1 -c4 -d30s -s scripts/remove.lua --latency http://localhost:8000/PBCB-a31835ad-a60a-4992-996b-57246fe50b11
Running 30s test @ http://localhost:8000/PBCB-a31835ad-a60a-4992-996b-57246fe50b11
  1 threads and 4 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    75.86us   70.73us   5.05ms   98.97%
    Req/Sec    55.36k     3.24k   67.31k    73.75%
  Latency Distribution
     50%   70.00us
     75%   80.00us
     90%   87.00us
     99%  152.00us
  1657242 requests in 30.10s, 224.43MB read
Requests/sec:  55057.76
Transfer/sec:      7.46MB
```
