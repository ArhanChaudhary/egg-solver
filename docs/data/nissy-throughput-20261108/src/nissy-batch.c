#define _GNU_SOURCE
#include "nissy.h"
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdatomic.h>
#include <string.h>
#include <errno.h>
#include <fcntl.h>
#include <sched.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/vfs.h>
#include <sys/wait.h>
#include <sys/resource.h>
#include <time.h>
#include <unistd.h>
#define MAX_CUBES 4096
#ifndef NISSY_COMPILED_THREADS
#define NISSY_COMPILED_THREADS 48
#endif
struct shared { _Atomic unsigned next,failures; };
static double now(void){struct timespec t;clock_gettime(CLOCK_MONOTONIC,&t);return t.tv_sec+t.tv_nsec*1e-9;}
static int fail(const char *s){fprintf(stderr,"%s: %s\n",s,strerror(errno));return 1;}
int main(int argc,char **argv){
 if(argc!=6){fprintf(stderr,"usage: nissy-batch SOLVER TABLE INPUT WORKERS THREADS_PER_CUBE\n");return 2;}
 unsigned workers=(unsigned)atoi(argv[4]),threads=(unsigned)atoi(argv[5]);
 if(!workers||!threads||workers*threads>48||threads>NISSY_COMPILED_THREADS){fprintf(stderr,"invalid CPU allocation\n");return 2;}
 const char *solver=argv[1];char dataid[NISSY_SIZE_DATAID];long long needed=nissy_solverinfo(solver,dataid);if(needed<=0)return 1;
 int fd=open(argv[2],O_RDONLY);struct stat st;struct statfs sf;
 if(fd<0||fstat(fd,&st)||fstatfs(fd,&sf))return fail("open table");
 if(st.st_size<needed||sf.f_type!=0x958458f6||sf.f_bsize!=(1L<<30)){fprintf(stderr,"table must be mapped from 1-GiB hugetlbfs and contain the complete data\n");return 1;}
 const unsigned char *data=mmap(NULL,(size_t)st.st_size,PROT_READ,MAP_SHARED,fd,0);if(data==MAP_FAILED)return fail("mmap table");
 if(nissy_checkdata(solver,(unsigned long long)needed,data)<0){fprintf(stderr,"invalid table\n");return 1;}
 char cubes[MAX_CUBES][NISSY_SIZE_CUBE];unsigned count=0;FILE *in=fopen(argv[3],"r");if(!in)return fail("input");
 char *line=NULL;size_t cap=0;ssize_t len;
 while((len=getline(&line,&cap,in))>=0){
  if(count==MAX_CUBES){fprintf(stderr,"too many input cubes\n");return 1;}
  char *q=line;for(char *p=line;*p;p++)if(*p!='1')*q++=*p;*q=0;
  if(nissy_applymoves(NISSY_SOLVED_CUBE,line,cubes[count])<0){fprintf(stderr,"invalid cube %u\n",count);return 1;}count++;
 }
 free(line);fclose(in);if(!count)return 1;
 struct shared *state=mmap(NULL,sizeof(*state),PROT_READ|PROT_WRITE,MAP_SHARED|MAP_ANONYMOUS,-1,0);if(state==MAP_FAILED)return fail("shared state");atomic_init(&state->next,0);atomic_init(&state->failures,0);
 nissy_setlogger(NULL,NULL);double start=now();unsigned started=0;
 for(unsigned worker=0;worker<workers;worker++){
  pid_t pid=fork();if(pid<0){atomic_fetch_add(&state->failures,1);break;}
  if(pid==0){
   cpu_set_t cpus;CPU_ZERO(&cpus);for(unsigned c=worker*threads;c<(worker+1)*threads;c++)CPU_SET(c,&cpus);
   if(sched_setaffinity(0,sizeof(cpus),&cpus))_exit(3);
   for(;;){unsigned i=atomic_fetch_add(&state->next,1);if(i>=count)break;
    char sols[NISSY_SIZE_MOVES]={0},replay[NISSY_SIZE_CUBE];long long stats[NISSY_SIZE_SOLVE_STATS]={0};
    double begin=now();long long rc=nissy_solve(cubes[i],solver,NISSY_NISSFLAG_NORMAL,0,20,1,0,threads,(unsigned long long)needed,data,sizeof(sols),sols,stats,NULL,NULL);double elapsed=now()-begin;
    char *end=strchr(sols,'\n');if(end)*end=0;
    long long length=nissy_countmoves(sols);int ok=rc>0&&length>=0&&nissy_applymoves(cubes[i],sols,replay)>=0&&!strcmp(replay,NISSY_SOLVED_CUBE);
    if(!ok)atomic_fetch_add(&state->failures,1);
    dprintf(STDOUT_FILENO,"{\"type\":\"solved\",\"index\":%u,\"worker\":%u,\"length\":%lld,\"nodes\":%lld,\"lookups\":%lld,\"fallbacks\":%lld,\"seconds\":%.9f,\"moves\":\"%s\",\"ok\":%s,\"returncode\":%lld}\n",i,worker,length,stats[0],stats[1],stats[2],elapsed,sols,ok?"true":"false",rc);
   }_exit(0);
  }started++;
 }
 for(unsigned i=0;i<started;i++){int status;pid_t p;do{p=wait(&status);}while(p<0&&errno==EINTR);if(p<0||!WIFEXITED(status)||WEXITSTATUS(status))atomic_fetch_add(&state->failures,1);}
 double wall=now()-start;struct rusage usage;getrusage(RUSAGE_CHILDREN,&usage);double cpu=usage.ru_utime.tv_sec+usage.ru_utime.tv_usec/1e6+usage.ru_stime.tv_sec+usage.ru_stime.tv_usec/1e6;
 printf("{\"type\":\"total\",\"cubes\":%u,\"wall\":%.9f,\"wall_per_cube\":%.9f,\"workers\":%u,\"threads_per_cube\":%u,\"child_cpu_seconds\":%.9f,\"failures\":%u,\"table_bytes\":%lld,\"page_bytes\":%ld}\n",count,wall,wall/count,workers,threads,cpu,atomic_load(&state->failures),needed,sf.f_bsize);
 return atomic_load(&state->failures)?1:0;
}
