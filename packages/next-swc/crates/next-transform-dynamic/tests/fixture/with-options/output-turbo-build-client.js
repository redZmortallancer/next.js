"TURBOPACK { chunking-type: none }";
import { __turbopack_module_id__ as id } from "../components/hello";
"TURBOPACK { chunking-type: none }";
import { __turbopack_module_id__ as id1 } from "../components/hello";
"TURBOPACK { chunking-type: none }";
import { __turbopack_module_id__ as id2 } from "../components/hello";
import dynamic from 'next/dynamic';
const DynamicComponentWithCustomLoading = dynamic(()=>import('../components/hello'), {
    loadableGenerated: {
        webpack: ()=>[
                id
            ]
    },
    loading: ()=><p>...</p>
});
const DynamicClientOnlyComponent = dynamic(()=>import('../components/hello'), {
    loadableGenerated: {
        webpack: ()=>[
                id1
            ]
    },
    ssr: false
});
const DynamicClientOnlyComponentWithSuspense = dynamic(()=>import('../components/hello'), {
    loadableGenerated: {
        webpack: ()=>[
                id2
            ]
    },
    ssr: false,
    suspense: true
});
