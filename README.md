# allm
Single frontend for all of the free LLMs

Tired of running up to free-tier limits on your LLM api calls? Here is the way to mitigate it!

The main purpose of this RUST library is to provide 
1) a unified frontend to all the free LLM APIs
2) automatically switch to another LLM provider and/or model when a request fails.

I'll be adding some secondary functionality: nice quality-of-life features, like 
3) multi-requests + adjudication: send a single prompt simultaneously to multiple different LLM provider / models, use another provider / model to decide the best one or synthisize the optimal response.


\- SEE
