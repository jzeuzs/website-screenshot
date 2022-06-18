FROM rustlang/rust:nightly-bullseye-slim as base

WORKDIR /usr/src/app

ENV CI=true
ENV RUSTFLAGS="-C target-cpu=native"

# Build stuff
RUN apt-get update && \
    apt-get upgrade -y --no-install-recommends && \
    apt-get install -y --no-install-recommends build-essential python3 gnupg wget curl unzip dumb-init clang lld libssl-dev pkg-config  && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    apt-get autoremove

# Chrome & Chromedriver
RUN wget -q -O - https://dl-ssl.google.com/linux/linux_signing_key.pub | apt-key add - && \
    sh -c 'echo "deb [arch=amd64] http://dl.google.com/linux/chrome/deb/ stable main" >> /etc/apt/sources.list.d/google-chrome.list' && \
    apt-get -y update && \
    apt-get install -y google-chrome-stable && \
    wget -O /tmp/chromedriver.zip http://chromedriver.storage.googleapis.com/`curl -sS chromedriver.storage.googleapis.com/LATEST_RELEASE`/chromedriver_linux64.zip && \
    unzip /tmp/chromedriver.zip chromedriver -d /usr/local/bin/

# Fleet
RUN cargo install fleet-rs && \
    cargo install sccache

ENV DISPLAY=:99
ENTRYPOINT ["dumb-init", "--"]

FROM base as builder

COPY Cargo.lock .
COPY Cargo.toml .
COPY build.rs .
COPY static/ static/
COPY openapi.yml .

RUN mkdir src && \
    echo "// blank" > src/lib.rs && \
    fleet build --release && \
    rm -r src

COPY src/ src/
COPY evasions/ evasions/

RUN fleet build --release

FROM base as runner

COPY --from=builder /usr/src/app/target/release/website-screenshot /usr/local/bin/website-screenshot

ENTRYPOINT ["chromedriver", "&", "/usr/local/bin/website-screenshot"]
