FROM jimmycuadra/rust

RUN apt-get update && \
    apt-get install -y xcb-proto libcairo2-dev libpango1.0-dev libpangocairo-1.0