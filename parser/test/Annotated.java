package org.acme;

import jakarta.ws.rs.GET;
import java.util.stream.IntStream;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;

@Path("/hello")
public class GreetingResource {
    private static final Long other;

    @Path("{name}")
    @GET
    @Produces(MediaType.TEXT_PLAIN)
    public String hello(String name) {
        name.chars().spliterator();
        return "Hello from Quarkus REST";
    }

    public static void other() {}
}
