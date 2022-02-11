from os.path import join

from benchmark.utils import PathMaker


class CommandMaker:

    @staticmethod
    def cleanup():
        return (
            f'rm -r .*-db* ; rm .*.json ; mkdir -p {PathMaker.results_path()}'
        )

    @staticmethod
    def clean_logs():
        return f'rm -r {PathMaker.logs_path()} ; mkdir -p {PathMaker.logs_path()}'

    @staticmethod
    def compile(witness_only):
        assert isinstance(witness_only, bool)
        feature = 'witness-only-benchmark' if witness_only else 'benchmark'
        return f'cargo build --quiet --release --features {feature}'

    @staticmethod
    def generate_key(key_file):
        assert isinstance(key_file, str)
        return f'./witness generate --filename {key_file}'

    @staticmethod
    def run_witness(keypair, committee, secure_store, audit_storage, debug=False):
        assert isinstance(keypair, str)
        assert isinstance(committee, str)
        assert isinstance(secure_store, str)
        assert isinstance(audit_storage, str)
        assert isinstance(debug, bool)
        v = '-vvv' if debug else '-vv'
        return (
            f'./witness {v} run --keypair {keypair} --committee {committee} '
            f'--secure_storage {secure_store} --audit_storage {audit_storage}'
        )

    @staticmethod
    def run_client(rate, idp, committee, proof_entries, witness_only, debug=False):
        assert isinstance(idp, str)
        assert isinstance(rate, int)
        assert isinstance(proof_entries, int)
        assert isinstance(committee, str)
        assert isinstance(witness_only, bool)
        assert isinstance(debug, bool)
        v = '-vvv' if debug else '-vv'
        bin = 'witness_client' if witness_only else 'idp_client'
        return (
            f'./{bin} {v} --idp {idp} --rate {rate} '
            f'--committee {committee} --proof_entries {proof_entries}'
        )

    @staticmethod
    def kill():
        return 'tmux kill-server'

    @staticmethod
    def alias_binaries(origin, witness_only):
        assert isinstance(origin, str)
        assert isinstance(witness_only, bool)
        client_bin = 'witness_client' if witness_only else 'idp_client'
        node = join(origin, 'witness')
        client = join(origin, client_bin)
        return (
            f'rm witness ; rm {client_bin}'
            f'; ln -s {node} . ; ln -s {client} .'
        )
