FROM frolvlad/alpine-glibc
MAINTAINER <Young Wu> doomsplayer@gmail.com

ENV TERM="xterm-256color" LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 LC_CTYPE=UTF-8

COPY target/x86_64-unknown-linux-gnu/release/nextaction /

CMD ["/nextaction"]
