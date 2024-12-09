open Sexplib.Std

let sexp_of_bytes bytes : Sexplib0.Sexp.t =
  let first_zero = Bytes.index_opt bytes '\000' in
  let first_zero = Option.value ~default:(Bytes.length bytes) first_zero in
  let str = Bytes.sub_string bytes 0 first_zero in
  let str = Str.global_replace (Str.regexp_string "\n") "\\n" str in
  let str = Str.global_replace (Str.regexp_string "\r") "\\r" str in
  sexp_of_string str

type http_method =
  | GET
  | HEAD
  | POST
  | PUT
  | DELETE
  | CONNECT
  | OPTIONS
  | TRACE
[@@deriving sexp_of]

type http_version = { major : int; minor : int } [@@deriving sexp_of]
type http_header = { name : string; value : bytes } [@@deriving sexp_of]

type http_request_parts = {
  rmethod : http_method;
  target : string;
  version : http_version;
  headers : http_header list;
}
[@@deriving sexp_of]

type parser = { buf : bytes; buf_len : int; buf_cap : int } [@@deriving sexp_of]

type parser_recv_req_err =
  | EXPECT_SP
  | EXPECT_NEWLINE
  | INVALID_METHOD
  | EMPTY_TARGET
  | INVALID_VERSION
  | INVALID_FIELD_LINE_NO_SEP
  | INVALID_FIELD_LINE_EMPTY_NAME
  | INVALID_FIELD_LINE_NAME_TRAILING_SP
  | INVALID_FIELD_LINE_EMPTY_VALUE
[@@deriving sexp_of]

type parser_recv_req_stage =
  | INIT
  | REQ_METHOD
  | REQ_TARGET of { rmethod : http_method }
  | REQ_VER of { rmethod : http_method; target : string }
  | REQ_FIELD of {
      rmethod : http_method;
      target : string;
      version : http_version;
    }
  | RESULT of http_request_parts
  | ERR of parser_recv_req_stage * parser_recv_req_err
[@@deriving sexp_of]

type parser_recv_req_state = {
  socket : (Unix.file_descr[@sexp.opaque]);
  p : parser;
  s : parser_recv_req_stage;
}
[@@deriving sexp_of]

let parser_create (buf_size : int) : parser =
  { buf = Bytes.create buf_size; buf_len = 0; buf_cap = buf_size }

let parser_recv (socket : Unix.file_descr) (p : parser) : parser =
  let size = Unix.recv socket p.buf p.buf_len (p.buf_cap - p.buf_len) [] in
  if size = 0 then raise (Failure "connection closed");
  { p with buf_len = p.buf_len + size }

let parser_skip (n : int) (p : parser) : parser =
  assert (n >= 0);
  assert (n <= p.buf_len);
  { p with buf_len = p.buf_len - n; buf = Bytes.extend p.buf (-n) n }

let parser_buf_cur (p : parser) : bytes = Bytes.sub p.buf 0 p.buf_len

let parser_recv_req_parts (socket : Unix.file_descr) (parser : parser) :
    parser_recv_req_state =
  let recv_once (state : parser_recv_req_state) : parser_recv_req_state =
    { state with p = parser_recv state.socket state.p }
  in

  let skip_n (n : int) (state : parser_recv_req_state) : parser_recv_req_state =
    { state with p = parser_skip n state.p }
  in

  let next_stage (next_stage : parser_recv_req_stage)
      (state : parser_recv_req_state) : parser_recv_req_state =
    { state with s = next_stage }
  in

  let strip_until (target : char) (state : parser_recv_req_state) :
      bytes * parser_recv_req_state =
    let rec strip_rec cur s =
      let need_more, len =
        match Bytes.index_opt s.p.buf target with
        | Some t_at -> (false, t_at)
        | None -> (true, s.p.buf_len)
      in
      let content = Bytes.concat Bytes.empty [ cur; Bytes.sub s.p.buf 0 len ] in
      let next_state = skip_n len s in
      if need_more then strip_rec content (recv_once next_state)
      else (content, next_state)
    in

    strip_rec Bytes.empty state
  in

  let strip_until_str (target : char) (state : parser_recv_req_state) :
      string * parser_recv_req_state =
    let content, new_state = strip_until target state in
    (Bytes.to_string content, new_state)
  in

  let rec strip_exact (target : bytes) (state : parser_recv_req_state) :
      bool * parser_recv_req_state =
    let target_len = Bytes.length target in

    assert (state.p.buf_cap >= target_len);

    if state.p.buf_len < target_len then
      if
        state.p.buf_len > 0
        && not (Bytes.starts_with ~prefix:(parser_buf_cur state.p) target)
      then (false, state)
      else strip_exact target (recv_once state)
    else if Bytes.starts_with ~prefix:target state.p.buf then
      (true, skip_n target_len state)
    else (false, state)
  in

  let strip_exact_str (target : string) (state : parser_recv_req_state) :
      bool * parser_recv_req_state =
    strip_exact (String.to_bytes target) state
  in

  let rec strip_n (n : int) (state : parser_recv_req_state) :
      bytes * parser_recv_req_state =
    assert (state.p.buf_cap >= n);

    if state.p.buf_len < n then strip_n n (recv_once state)
    else (Bytes.sub state.p.buf 0 n, skip_n n state)
  in

  let strip_newline state =
    let success, new_state = strip_exact_str "\n" state in
    if success then (success, new_state) else strip_exact_str "\r\n" new_state
  in

  let strip_http_method (state : parser_recv_req_state) :
      http_method option * parser_recv_req_state =
    let try_strip (method_str : string) (rmethod : http_method)
        (prev_state : http_method option * parser_recv_req_state) =
      if Option.is_some (fst prev_state) then prev_state
      else
        let success, new_state = strip_exact_str method_str (snd prev_state) in
        if success then (Some rmethod, new_state)
        else (fst prev_state, new_state)
    in

    try_strip "GET" GET (None, state)
    |> try_strip "HEAD" HEAD |> try_strip "POST" POST |> try_strip "PUT" PUT
    |> try_strip "DELETE" DELETE
    |> try_strip "CONNECT" CONNECT
    |> try_strip "OPTIONS" OPTIONS
    |> try_strip "TRACE" TRACE
  in

  let rec recv_rec s : parser_recv_req_state =
    let expect_sp_then (rthen : parser_recv_req_state -> parser_recv_req_state)
        (state : parser_recv_req_state) =
      let success, new_state = strip_exact_str " " state in
      if success then rthen new_state else { s with s = ERR (s.s, EXPECT_SP) }
    in

    let expect_newline_then
        (rthen : parser_recv_req_state -> parser_recv_req_state)
        (state : parser_recv_req_state) =
      let success, new_state = strip_newline state in
      if success then rthen new_state
      else { s with s = ERR (s.s, EXPECT_NEWLINE) }
    in

    match s.s with
    | INIT ->
        let rec strip_prefixing_newline state =
          let success, new_state = strip_newline state in
          if success then strip_prefixing_newline new_state else new_state
        in
        let s = strip_prefixing_newline s in
        recv_rec { s with s = REQ_METHOD }
    | REQ_METHOD -> (
        let method_opt, new_state = strip_http_method s in
        match method_opt with
        | Some rmethod ->
            expect_sp_then
              (fun new_state ->
                recv_rec (next_stage (REQ_TARGET { rmethod }) new_state))
              new_state
        | None -> { s with s = ERR (s.s, INVALID_METHOD) })
    | REQ_TARGET stage ->
        let target, new_state = strip_until_str ' ' s in
        if String.length target = 0 then { s with s = ERR (s.s, EMPTY_TARGET) }
        else
          expect_sp_then
            (fun new_state ->
              recv_rec
                (next_stage
                   (REQ_VER { rmethod = stage.rmethod; target })
                   new_state))
            new_state
    | REQ_VER stage ->
        let success, new_state = strip_exact_str "HTTP/" s in
        if not success then { s with s = ERR (s.s, INVALID_VERSION) }
        else
          let ver, new_state = strip_n 3 new_state in
          let ver = Bytes.to_string ver in
          let is_digit ch = ch >= '0' && ch <= '9' in
          let digit_to_int digit = Char.code digit - Char.code '0' in

          let major = ver.[0] in
          let minor = ver.[2] in
          if (not (is_digit major)) || ver.[1] != '.' || not (is_digit minor)
          then { s with s = ERR (s.s, INVALID_VERSION) }
          else
            let version =
              { major = digit_to_int major; minor = digit_to_int minor }
            in
            expect_newline_then
              (fun new_state ->
                recv_rec
                  (next_stage
                     (REQ_FIELD
                        {
                          rmethod = stage.rmethod;
                          target = stage.target;
                          version;
                        })
                     new_state))
              new_state
    | REQ_FIELD stage ->
        let strip_field_line state =
          let is_end, new_state = strip_newline state in

          if is_end then (true, Bytes.empty, new_state)
          else
            let line, new_state = strip_until '\n' new_state in
            let new_state = skip_n 1 new_state in
            let len = Bytes.length line in
            (* not newline only, LF *)
            assert (len > 0);
            let ends_with_cr =
              Bytes.ends_with ~suffix:(String.to_bytes "\r") line
            in
            (* not newline only, CRLF *)
            assert (not (len = 1 && ends_with_cr));
            let line_content =
              if ends_with_cr then Bytes.sub line 0 (len - 1) else line
            in
            (false, line_content, new_state)
        in

        let rec strip_headers hdrs state =
          let create_err state err =
            (true, [], { state with s = ERR (state.s, err) })
          in
          let is_end, line, new_state = strip_field_line state in

          if is_end then (false, hdrs, new_state)
          else
            let len = Bytes.length line in
            assert (len > 0);
            match Bytes.index_opt line ':' with
            | None -> create_err new_state INVALID_FIELD_LINE_NO_SEP
            | Some idx ->
                if idx = 0 then
                  create_err new_state INVALID_FIELD_LINE_EMPTY_NAME
                else
                  let name = Bytes.sub_string line 0 idx in
                  let value = Bytes.sub line (idx + 1) (len - idx - 1) in

                  let value =
                    Bytes.fold_left
                      (fun aac cur ->
                        if Bytes.length aac > 0 then
                          Bytes.concat Bytes.empty [ aac; Bytes.make 1 cur ]
                        else if cur = '\t' || cur = ' ' then aac
                        else Bytes.make 1 cur)
                      Bytes.empty value
                  in
                  let value =
                    Bytes.fold_right
                      (fun cur aac ->
                        if Bytes.length aac > 0 then
                          Bytes.concat Bytes.empty [ Bytes.make 1 cur; aac ]
                        else if cur = '\t' || cur = ' ' then aac
                        else Bytes.make 1 cur)
                      value Bytes.empty
                  in
                  if String.ends_with ~suffix:" " name then
                    create_err new_state INVALID_FIELD_LINE_NAME_TRAILING_SP
                  else if Bytes.length value = 0 then
                    create_err new_state INVALID_FIELD_LINE_EMPTY_VALUE
                  else strip_headers (hdrs @ [ { name; value } ]) new_state
        in

        let is_err, hdrs, new_state = strip_headers [] s in

        if is_err then new_state
        else
          {
            new_state with
            s =
              RESULT
                {
                  rmethod = stage.rmethod;
                  target = stage.target;
                  version = stage.version;
                  headers = hdrs;
                };
          }
    | RESULT _ -> s
    | ERR _ -> s
  in
  recv_rec { socket; p = parser; s = INIT }

let rec header_get_opt (name : string) (headers : http_header list) :
    http_header option =
  match headers with
  | [] -> None
  | cur :: rest ->
      if cur.name = name then Some cur else header_get_opt name rest

let string_of_sockaddr (addr : Unix.sockaddr) =
  match addr with
  | ADDR_UNIX addr -> addr
  | ADDR_INET (addr, port) ->
      Printf.sprintf "%s:%d" (Unix.string_of_inet_addr addr) port

let rec run_server (socket : Unix.file_descr) =
  let req_socket, addr = Unix.accept ~cloexec:true socket in

  Printf.printf "accept from: %s\n" (string_of_sockaddr addr);
  let req = parser_create 1024 |> parser_recv_req_parts req_socket in

  print_endline (Sexplib.Sexp.to_string_hum (sexp_of_parser_recv_req_state req));
  let parts =
    match req.s with
    | RESULT parts -> parts
    | _ -> raise (Failure "Failed to recv request parts")
  in
  let ua =
    match header_get_opt "User-Agent" parts.headers with
    | Some hdr -> Bytes.to_string hdr.value
    | None -> raise (Failure "Failed to get ua")
  in
  let host =
    match header_get_opt "Host" parts.headers with
    | Some hdr -> Bytes.to_string hdr.value
    | None -> raise (Failure "Failed to get host")
  in

  let res_body =
    Printf.sprintf
      "You are sending request to %s\n\
       You are requesting resource: %s\n\
       Your User Agent is %s\n"
      host parts.target ua
  in
  (*let res_body = "hello" in*)
  let res =
    String.to_bytes
      (Printf.sprintf
         "HTTP/1.1 200 Ok\r\n\
          Content-Length: %d\r\n\
          Content-Type: text/plain\r\n\
          Connection: close\r\n\
          \r\n\
          %s"
         (String.length res_body) res_body)
  in
  let _ = Unix.write req.socket res 0 (Bytes.length res) in
  Unix.close req.socket;
  run_server socket

let () =
  let listen_addr = Unix.ADDR_INET (Unix.inet_addr_of_string "0.0.0.0", 8085) in
  let socket = Unix.socket ~cloexec:true PF_INET SOCK_STREAM 0 in
  Unix.setsockopt socket SO_REUSEADDR true;

  Unix.bind socket listen_addr;
  Unix.listen socket 1;

  Printf.printf "listening on %s\n" (string_of_sockaddr listen_addr);
  flush stdout;

  run_server socket
