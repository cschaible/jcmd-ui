FROM rust:1.81 as builder

ADD jcmd-parser/ /usr/src/jcmd-ui/jcmd-parser
ADD jcmd-data-collector/ /usr/src/jcmd-ui/jcmd-data-collector
ADD tui/ /usr/src/jcmd-ui/tui

WORKDIR /usr/src/jcmd-ui/tui
RUN cargo build --release

FROM eclipse-temurin:21-jdk
RUN apt-get update && apt-get install -y jattach
COPY --from=builder /usr/src/jcmd-ui/tui/target/release/jcmd_tui /usr/bin/jcmd_tui
CMD ["/bin/bash"]
