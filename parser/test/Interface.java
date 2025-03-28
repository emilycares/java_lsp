package ch.emilycares;

import jdk.net.Sockets;
import java.io.IOException;
import java.net.Socket;public interface Constants {

    public String CONSTANT_A = "A";
    String CONSTANT_B = "B";

    String CONSTANT_C = "C";

    void display();

    Socket createSocket(String hostname, int port) throws IOException;
}
