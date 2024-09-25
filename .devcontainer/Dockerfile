FROM jetpackio/devbox-root-user:latest

# Installing your devbox project
WORKDIR /code
COPY devbox.json devbox.json
COPY devbox.lock devbox.lock



RUN devbox run -- echo "Installed Packages."

RUN devbox shellenv --init-hook >> ~/.profile
