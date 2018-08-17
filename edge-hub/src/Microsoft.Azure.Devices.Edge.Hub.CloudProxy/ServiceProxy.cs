// Copyright (c) Microsoft. All rights reserved.
namespace Microsoft.Azure.Devices.Edge.Hub.CloudProxy
{
    using System.Collections.Generic;
    using System.Linq;
    using System.Threading.Tasks;
    using Microsoft.Azure.Devices.Edge.Hub.Core.Cloud;
    using Microsoft.Azure.Devices.Edge.Util;

    public class ServiceProxy : IServiceProxy
    {
        readonly ISecurityScopesApiClient securityScopesApiClient;

        public ServiceProxy(ISecurityScopesApiClient securityScopesApiClient)
        {
            this.securityScopesApiClient = Preconditions.CheckNotNull(securityScopesApiClient, nameof(securityScopesApiClient));
        }

        public ISecurityScopeIdentitiesIterator GetSecurityScopeIdentitiesIterator() => new SecurityScopeIdentitiesIterator(this.securityScopesApiClient);

        public async Task<Option<ServiceIdentity>> GetServiceIdentity(string deviceId)
        {
            ScopeResult scopeResult = await this.securityScopesApiClient.GetIdentity(deviceId, null);
            if (scopeResult != null)
            {
                if (scopeResult.Devices != null)
                {
                    int count = scopeResult.Devices.Count();
                    if (count == 1)
                    {
                        ServiceIdentity serviceIdentity = DeviceToServiceIdentity(scopeResult.Devices.First());
                        return Option.Some(serviceIdentity);
                    }
                }
                else
                {
                    // Log More than one device received
                }
            }
            else
            {
                // Log null scope result
            }

            return Option.None<ServiceIdentity>();
        }

        public async Task<Option<ServiceIdentity>> GetServiceIdentity(string deviceId, string moduleId)
        {
            ScopeResult scopeResult = await this.securityScopesApiClient.GetIdentity(deviceId, moduleId);
            if (scopeResult != null)
            {
                if (scopeResult.Modules != null)
                {
                    int count = scopeResult.Modules.Count();
                    if (count == 1)
                    {
                        ServiceIdentity serviceIdentity = ModuleToServiceIdentity(scopeResult.Modules.First());
                        return Option.Some(serviceIdentity);
                    }
                }
                else
                {
                    // Log More than one device received
                }
            }
            else
            {
                // Log null scope result
            }

            return Option.None<ServiceIdentity>();
        }

        static ServiceIdentity DeviceToServiceIdentity(Device device)
        {
            return new ServiceIdentity(
                device.Id,
                null,
                device.Capabilities?.IotEdge ?? false,
                GetServiceAuthentication(device.Authentication));
        }

        static ServiceIdentity ModuleToServiceIdentity(Module module)
        {
            return new ServiceIdentity(
                module.Id,
                null,
                false,
                GetServiceAuthentication(module.Authentication));
        }

        static ServiceAuthentication GetServiceAuthentication(AuthenticationMechanism authenticationMechanism)
        {
            switch (authenticationMechanism.Type)
            {
                case Devices.AuthenticationType.CertificateAuthority:
                    return new ServiceAuthentication(ServiceAuthenticationType.CertificateAuthority, null, null);

                case Devices.AuthenticationType.SelfSigned:
                    return new ServiceAuthentication(ServiceAuthenticationType.CertificateThumbprint, null,
                        new X509Thumbprint(authenticationMechanism.X509Thumbprint.PrimaryThumbprint, authenticationMechanism.X509Thumbprint.SecondaryThumbprint));

                case Devices.AuthenticationType.Sas:
                    return new ServiceAuthentication(ServiceAuthenticationType.SasKey, new SymmetricKey(authenticationMechanism.SymmetricKey.PrimaryKey, authenticationMechanism.SymmetricKey.SecondaryKey), null);

                default:
                    return new ServiceAuthentication(ServiceAuthenticationType.None, null, null);
            }
        }

        class SecurityScopeIdentitiesIterator : ISecurityScopeIdentitiesIterator
        {
            readonly ISecurityScopesApiClient securityScopesApiClient;
            Option<string> continuationLink = Option.None<string>();

            public SecurityScopeIdentitiesIterator(ISecurityScopesApiClient securityScopesApiClient)
            {
                this.securityScopesApiClient = Preconditions.CheckNotNull(securityScopesApiClient, nameof(securityScopesApiClient));
                this.HasNext = true;
            }

            public async Task<IEnumerable<ServiceIdentity>> GetNext()
            {
                var serviceIdentities = new List<ServiceIdentity>();
                ScopeResult scopeResult = await this.continuationLink.Map(c => this.securityScopesApiClient.GetNext(c))
                    .GetOrElse(() => this.securityScopesApiClient.GetIdentitiesInScope());
                serviceIdentities.AddRange(scopeResult.Devices.Select(d => DeviceToServiceIdentity(d)));
                serviceIdentities.AddRange(scopeResult.Modules.Select(m => ModuleToServiceIdentity(m)));

                if (!string.IsNullOrWhiteSpace(scopeResult.ContinuationLink))
                {
                    this.continuationLink = Option.Some(scopeResult.ContinuationLink);
                    this.HasNext = true;
                }
                else
                {
                    this.HasNext = false;
                }

                return serviceIdentities;
            }

            public bool HasNext { get; private set; }            
        }
    }
}
