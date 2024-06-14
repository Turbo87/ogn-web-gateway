Contributing
==============================================================================


Docker Quickstart
------------------------------------------------------------------------------

Although docker is not used as the primary means of deploying ogn-web-gateway
it can be very helpful as a test environment. With docker installed and this
repo cloned, it's as easy as `docker-compose up`. You can easily verify that
it is up with:
```bash
curl http://0.0.0.0:8080/api/status
```
