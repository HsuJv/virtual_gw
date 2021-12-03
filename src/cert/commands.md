# Self-signed CA & Certs
01. Generate CA

```Bash
openssl req -new -x509 -nodes -days 365 -subj '/CN=CA' -keyout ca.key -out ca.crt
```

02. Generate Server Key

```Bash
openssl genrsa -aes256 -out ./server-key.pem 2048
openssl rsa -in ./server-key.pem -out ./server-key.pem
```

03. Generate Server CSR

```Bash
openssl req -new -key ./server-key.pem -out ./server.csr -subj "/C=CN/ST=Shanghai/L=Shanghai/O=/OU=/CN=Server"
```

04. Sign the Server CSR

```Bash
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -days 365 -out server.crt
```

05. Generate Client Key

```Bash
openssl genrsa -aes256 -out ./client-key.pem 2048
openssl rsa -in ./client-key.pem -out ./client-key.pem
```

06. Generate Client CSR

```Bash
openssl req -new -key ./client-key.pem -out ./client.csr -subj "/C=CN/ST=Shanghai/L=Shanghai/O=/OU=/CN=Client"
```

07. Sign Client Key

```Bash
openssl x509 -req -in client.csr -CA ca.crt -CAkey ca.key -CAcreateserial -days 365 -out client.crt
```
