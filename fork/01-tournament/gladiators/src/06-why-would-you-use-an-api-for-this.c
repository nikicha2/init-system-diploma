#define _POSIX_C_SOURCE 200112L
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netdb.h>
#include <arpa/inet.h>

int main(void) {
    // Configuration
    char *hostname = "api.mathjs.org";
    char *port = "80"; // HTTP (without s)
    char *path = "/v4/?expr=randomInt(1,128)"; // set number range

    struct addrinfo hints, *res;
    int sockfd;
    char buffer[4096];

    printf("--- Connecting to an API for a single integer ---\n");
    printf("---         (why would one do this?!?)        ---\n");

    // DNS Lookup
    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_UNSPEC;     // IPv4 or IPv6
    hints.ai_socktype = SOCK_STREAM; // TCP

    if (getaddrinfo(hostname, port, &hints, &res) != 0) {
        perror("DNS lookup failed");
        return 1;
    }

    // Socket creation & connection
    sockfd = socket(res->ai_family, res->ai_socktype, res->ai_protocol);
    if (sockfd == -1) {
        perror("Socket creation failed");
        return 1;
    }

    if (connect(sockfd, res->ai_addr, res->ai_addrlen) == -1) {
        perror("Connection failed");
        return 1;
    }

    printf("Connected! Requesting randomness...\n");

    // HTTP Request
    // Using HTTP/1.0 to close the connection immediately after the response
    snprintf(buffer, sizeof(buffer),
             "GET %s HTTP/1.0\r\n"
             "Host: %s\r\n"
             "User-Agent: C-Socket-Client\r\n"
             "\r\n", 
             path, hostname);

    // Send the request
    if (send(sockfd, buffer, strlen(buffer), 0) == -1) {
        perror("Send failed");
        return 1;
    }

    // Parse response
    // We loop because TCP data can arrive in chunks
    int bytes_received;
    int header_ended = 0;
    int random_number;
    
    printf("\n--- SERVER RESPONSE ---\n");
    
    while ((bytes_received = recv(sockfd, buffer, sizeof(buffer) - 1, 0)) > 0) {
        buffer[bytes_received] = '\0'; // Null-terminate the data
        
        // Very basic logic to skip headers and print only the body (the number)
        // We look for the double newline \r\n\r\n that separates headers from body
        if (!header_ended) {
            char *body_start = strstr(buffer, "\r\n\r\n");
            if (body_start) {
                header_ended = 1;
                random_number = atoi(body_start + 4);
                printf("Your Random Number is: %d.", random_number);
            }
        } else {
            printf("%s", buffer);
        }
    }

    printf("\n-----------------------\n");

    // Cleanup
    close(sockfd);
    freeaddrinfo(res);

    return random_number;
}
