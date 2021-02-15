FROM rust:latest
RUN mkdir /app
WORKDIR /tmp
COPY . /tmp
RUN cargo install --path .
WORKDIR /app
ENTRYPOINT ["f6tgbot"]