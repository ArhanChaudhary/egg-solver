"""Repair optional annotation export without changing measured solver code."""
import ast


def patch_runner(text):
    before = '''        self.execute([self.perf,'annotate','--stdio','--no-source','-i',data,'--symbol','bench::stream_thread'],
                     name='record/'+base.name+'.annotate.txt',timeout=180)
        if not read(str(base)+'.annotate.txt').strip():
'''
    after = '''        annotation_code=self.execute([self.perf,'annotate','--stdio','--no-source','-i',data,'--symbol','bench::stream_thread'],
                     name='record/'+base.name+'.annotate.txt',timeout=180)
        annotation=read(str(base)+'.annotate.txt')
        if annotation_code!=0 or not annotation.strip() or 'no samples!' in annotation:
'''
    if text.count(before) != 1:
        raise RuntimeError('PMU annotation patch no longer matches the original runner')
    result = text.replace(before, after)
    before_samples = '''        self.execute([self.perf,'script','-i',data,'-F','comm,pid,tid,cpu,time,event,ip,sym,dso'],
                     name='record/'+base.name+'.samples.txt',timeout=180)
'''
    after_samples = '''        sample_code=self.execute([self.perf,'script','-i',data,'-F','comm,pid,tid,cpu,time,event,ip,sym,dso'],
                     name='record/'+base.name+'.samples.txt',timeout=180)
        sample_text=read(str(base)+'.samples.txt')
        if sample_code!=0 or 'Cannot print' in sample_text:
            self.execute([self.perf,'script','-i',data,'-F','comm,pid,tid,time,event,ip,sym,dso'],
                         name='record/'+base.name+'.samples-no-cpu.txt',timeout=180)
'''
    if result.count(before_samples) != 1:
        raise RuntimeError('PMU sample export patch no longer matches the original runner')
    result = result.replace(before_samples, after_samples)
    ast.parse(result)
    return result
