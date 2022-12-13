export type track_def_t = {
    port: number,
    ip: string,
    codec: string
};

export type stream_def_t = {
    id: string,
    default: boolean,
    video?: track_def_t,
    audio?: track_def_t
}[];

export const API_STREAMS = "/api/streams";


export type stats_t = {
    system_status: {
        mem_total: number,
        mem_used: number,
        proc_mem: number,
        cpu_num: number,
        cpu_used: number,
        proc_cpu: number,
        uptime: number,
        proc_id: number,
    },
    clients: number
}

export const API_STATS = "/api/stats";
