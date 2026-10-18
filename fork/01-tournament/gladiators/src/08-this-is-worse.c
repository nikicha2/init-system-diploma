#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/wait.h>
#include <string.h>

int main(void) {
    int pipefd[2]; 
    pid_t pid;
    char buffer[128];
    int random_int = -1;

    char *cmd = "curl -s -X POST https://os-homework-ollama.penchev.dev/api/generate "
                "-H \"Content-Type: application/json\" "
                "-d '{\"model\": \"llama3.2:1b\", \"prompt\": \"Generate a random integer strictly between 0 and 99. Output ONLY the digits. No words, no punctuation, no nothing.\", \"stream\": false}' "
                "| jq -r .response";

    if (pipe(pipefd) == -1) {
        perror("pipe");
        exit(1);
    }

    pid = fork();

    if (pid == -1) {
        perror("fork");
        exit(1);
    }

    if (pid == 0) {
        close(pipefd[0]); 
        dup2(pipefd[1], STDOUT_FILENO);
        close(pipefd[1]);

        execl("/bin/sh", "sh", "-c", cmd, (char *) NULL);

        perror("execl");
        exit(1);
    } else {
        close(pipefd[1]); 
        
        int status;
        waitpid(pid, &status, 0);

        int nbytes = read(pipefd[0], buffer, sizeof(buffer) - 1);
        
        if (nbytes > 0) {
            buffer[nbytes] = '\0'; 
            random_int = atoi(buffer);
            
            printf("Received: %d\n", random_int);
            return random_int;
        } else {
            printf("No data received.\n");
            return 1;
        }

        close(pipefd[0]);
    }

    return 0;
}
