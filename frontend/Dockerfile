# The SvelteKit node server (adapter-node build).

FROM node:22-bookworm-slim AS builder
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:22-bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production
COPY --from=builder /app/build build
COPY --from=builder /app/package.json ./
USER node
EXPOSE 3000
CMD ["node", "build"]
