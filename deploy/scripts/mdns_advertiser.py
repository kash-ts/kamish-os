import socket
import time
from zeroconf import IPVersion, ServiceInfo, Zeroconf

def advertise():
    ip_address = "192.168.1.184"
    port = 8080
    service_name = "Kamish Web Panel._http._tcp.local."
    
    info = ServiceInfo(
        "_http._tcp.local.",
        service_name,
        addresses=[socket.inet_aton(ip_address)],
        port=port,
        properties={"path": "/"},
        server="kamish.local.",
    )

    zeroconf = Zeroconf(ip_version=IPVersion.V4Only)
    print(f"Broadcasting mDNS: http://kamish.local:{port} -> {ip_address}")
    zeroconf.register_service(info)

    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        pass
    finally:
        zeroconf.unregister_service(info)
        zeroconf.close()

if __name__ == "__main__":
    advertise()
