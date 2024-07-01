# Start from the official Ubuntu 24.04 image
FROM ubuntu:24.04

# Install required packages
RUN apt-get update && \
    apt-get install -y \
        build-essential \
        libgl1-mesa-dev \
        mesa-common-dev \
        libsdl2-dev \
        libglm-dev \
        git \
        cmake \
        ca-certificates \
        curl \
        sudo

# Clone the projectM repository and build it
RUN git clone --depth 1 https://github.com/projectM-visualizer/projectm.git /tmp/projectm && \
    cd /tmp/projectm && \
    git fetch --all --tags && \
    git submodule init && \
    git submodule update && \
    mkdir build && \
    cd build && \
    cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr/local .. && \
    make -j8 && \
    make install && \
    rm -rf /tmp/projectm

# Set environment variables for projectM and the GStreamer plugin
ENV PROJECTM_ROOT=/usr/local
ENV GST_PLUGIN_PATH=/var/task/projectm/gstreamer-plugins

# Clone the gst-projectm repository and build the GStreamer plugin
RUN git clone https://github.com/projectM-visualizer/gst-projectm.git /tmp/gst-projectm
WORKDIR /tmp/gst-projectm
RUN ./setup.sh --auto
RUN mkdir build && \
    cd build && \
    cmake -DCMAKE_BUILD_TYPE=Release .. && \
    make
RUN ls
RUN pwd
RUN ls
RUN mkdir -p /var/task/projectm/gstreamer-plugins && \
    mv build/libgstprojectm.so $GST_PLUGIN_PATH/ && \
    rm -rf /tmp/gst-projectm

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
ENV PATH="/root/.cargo/bin:${PATH}"

# Create the Rust application
WORKDIR /usr/src/projectm_lambda
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Set the Lambda runtime entry point
COPY entrypoint.sh /var/task/entrypoint.sh
RUN chmod +x /var/task/entrypoint.sh

ENV RUST_LOG=debug
ENV RUST_BACKTRACE=1

ENTRYPOINT ["/var/task/entrypoint.sh"]

# Clean up unnecessary packages to reduce image size
RUN apt-get remove -y \
        build-essential \
        git \
        cmake && \
    apt-get autoremove -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
